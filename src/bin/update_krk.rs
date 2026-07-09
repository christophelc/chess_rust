use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::ops::BitAnd;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::entity::game::component::bitboard::piece_move::GenMoves;
use actix::Actor;
use chess_actix::entity;
use chess_actix::entity::engine::component::config::config_params;
use chess_actix::entity::engine::component::engine_mat::{self, EngineMat};
use chess_actix::entity::engine::component::evaluation::stat_eval;
use chess_actix::entity::game::actor::game_manager;
use chess_actix::entity::game::component::bitboard::{
    self, piece_move, zobrist, BitBoardsWhiteAndBlack,
};
use chess_actix::entity::game::component::{game_state, square};
use chess_actix::monitoring::debug;
use chess_actix::trace::init_trace;
use chess_actix::ui::notation;
use chess_actix::ui::notation::fen::{self, EncodeUserInput, Position, PositionStatus};
use chess_actix::ui::notation::long_notation::LongAlgebricNotationMove;
use entity::engine::actor::engine_dispatcher as dispatcher;

type FenLineMap = HashMap<zobrist::ZobristHash, FenLineEnriched>;

#[derive(Debug, Clone)]
struct FenLine {
    fen: String,
    mat_in: String,
    move_str: String,
}

#[derive(Debug, Clone)]
struct FenLineEnriched {
    fen_line: FenLine,
    hash: zobrist::ZobristHash,
}

impl std::fmt::Display for FenLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{};{}", self.fen, self.mat_in, self.move_str)
    }
}

fn find_white_move(start_position: &Position, end_position: &Position) -> bitboard::BitBoardMove {
    let binding = bitboard::BitPosition::from(*start_position);
    let start_position = binding.bit_boards_white_and_black().bit_board_white();
    let binding = bitboard::BitPosition::from(*end_position);
    let end_position = binding.bit_boards_white_and_black().bit_board_white();
    assert!(start_position != end_position);

    let diff = end_position.xor(start_position);
    let diff_boards = vec![
        diff.rooks().bitboard(),
        diff.bishops().bitboard(),
        diff.knights().bitboard(),
        diff.queens().bitboard(),
        diff.king().bitboard(),
        diff.pawns().bitboard(),
    ];
    let type_pieces = vec![
        square::TypePiece::Rook,
        square::TypePiece::Bishop,
        square::TypePiece::Knight,
        square::TypePiece::Queen,
        square::TypePiece::King,
        square::TypePiece::Pawn,
    ];
    let (diff_board, type_piece) = diff_boards
        .into_iter()
        .zip(type_pieces)
        .find(|(board, _)| board.non_empty())
        .unwrap();
    bitboard::BitBoardMove::new(
        square::Color::White,
        type_piece,
        start_position
            .concat_bit_boards()
            .bitand(*diff_board)
            .index(),
        end_position.concat_bit_boards().bitand(*diff_board).index(),
        None,
        None,
    )
}

fn play_move(
    game_state: &mut game_state::GameState,
    b_move: &bitboard::BitBoardMove,
    zobrist_table: &zobrist::Zobrist,
) {
    let long_algebraic_move =
        notation::long_notation::LongAlgebricNotationMove::build_from_b_move(*b_move);
    let _ = game_state.play_moves(&[long_algebraic_move], zobrist_table, None, false);
}

fn check_black_moves_lead_to_mat(
    game: &mut game_state::GameState,
    b_moves: &[bitboard::BitBoardMove],
    zobrist_table: &zobrist::Zobrist,
    known_positions: &FenLineMap,
) -> bool {
    if b_moves.len() == 1 {
        return true;
    };
    b_moves
        .iter()
        .all(|b_move| check_black_move_lead_to_mat(game, b_move, zobrist_table, known_positions))
}

fn check_black_move_lead_to_mat(
    game: &mut game_state::GameState,
    b_move: &bitboard::BitBoardMove,
    zobrist_table: &zobrist::Zobrist,
    known_positions: &FenLineMap,
) -> bool {
    play_move(game, b_move, zobrist_table);
    let hash = game.last_hash();
    game.play_back();
    known_positions.contains_key(&hash)
}

async fn try_reach_position(
    max_depth: u8,
    mat_in: u8,
    start_position: &Position,
    maybe_end_position: Option<&Position>,
    engine_player1: &EngineMat,
    known_positions: &FenLineMap,
    self_actor: actix::Addr<dispatcher::EngineDispatcher>,
    zobrist_table: &zobrist::Zobrist,
) -> Option<(u8, bitboard::BitBoardMove)> {
    let mut game = game_state::GameState::new(*start_position, zobrist_table);
    if let Some(&end_position) = maybe_end_position {
        assert!(mat_in > 1);
        let b_move = find_white_move(start_position, &end_position);
        play_move(&mut game, &b_move, zobrist_table);
        let black_moves = game.gen_moves();
        let result = if check_black_moves_lead_to_mat(
            &mut game,
            &black_moves,
            zobrist_table,
            known_positions,
        ) {
            Some((mat_in, b_move))
        } else {
            None
        };
        return result;
    }
    let mut stat_eval = stat_eval::StatEval::default();
    let flag_stop = Arc::new(AtomicBool::new(false));
    let mat_move_opt = engine_player1.mat_solver_init(
        &game,
        self_actor,
        None,
        &config_params::MatConfig::new(max_depth),
        &mut stat_eval,
        &flag_stop,
    );
    mat_move_opt
        .into_iter()
        .filter_map(|mat_move| {
            if !mat_move.is_position_reached() && mat_move.mat_in() == mat_in {
                Some((mat_move.mat_in(), *mat_move.bitboard_move()))
            } else {
                None
            }
        })
        .next()
}

fn extract_fields(line: &str) -> FenLine {
    let mut parts = line.split(';');
    let fen = parts.next().unwrap_or("").to_string();
    let mat_in = parts.next().unwrap_or("").to_string();
    let move_str = parts.next().unwrap_or("").to_string();
    FenLine {
        fen,
        mat_in,
        move_str,
    }
}

async fn update_line_mat_in_one(
    line: &str,
    engine_player1: &EngineMat,
    self_actor: actix::Addr<dispatcher::EngineDispatcher>,
    max_depth: u8,
    zobrist_table: &zobrist::Zobrist,
) -> String {
    let FenLine {
        fen,
        mat_in,
        move_str: _,
    } = extract_fields(line);
    if mat_in.is_empty() {
        let position = fen::Fen::decode(&fen).expect("Failed to decode FEN");
        if let Some((mat_in, bitboard_move)) = try_reach_position(
            max_depth,
            1,
            &position,
            None,
            engine_player1,
            &HashMap::<zobrist::ZobristHash, FenLineEnriched>::new(),
            self_actor,
            zobrist_table,
        )
        .await
        {
            let move_str = LongAlgebricNotationMove::build_from_b_move(bitboard_move).cast();
            let fen_line = FenLine {
                fen: fen.to_string(),
                mat_in: mat_in.to_string(),
                move_str,
            };
            fen_line.to_string()
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    }
}

// take input file and update it with position from which mat in 1 can be achieved in one move, and write to output file
async fn init_update_file(
    input: &str,
    output: &str,
    engine_player1: &EngineMat,
    self_actor: actix::Addr<dispatcher::EngineDispatcher>,
    zobrist_table: &zobrist::Zobrist,
) -> std::io::Result<()> {
    let infile = File::open(input)?;
    let reader = BufReader::new(infile);

    let outfile = File::create(output)?;
    let mut writer = BufWriter::new(outfile);

    for (i, line) in reader.lines().enumerate() {
        let line = line?;

        if i == 0 {
            // header
            writeln!(writer, "{}", line)?;
            continue;
        }

        let updated =
            update_line_mat_in_one(&line, engine_player1, self_actor.clone(), 1, zobrist_table)
                .await;
        writeln!(writer, "{}", updated)?;
    }

    writer.flush()?;
    Ok(())
}

fn copy_file(source: &str, dest: &str) -> std::io::Result<()> {
    std::fs::copy(source, dest)?;
    Ok(())
}

fn backup_filename(filename: &str) -> String {
    format!("{}.bkp", filename)
}

fn to_fen_lines_enriched(
    lines: &[String],
    zobrist_table: &zobrist::Zobrist,
) -> Vec<FenLineEnriched> {
    let mut v: Vec<FenLineEnriched> = vec![];
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            continue;
        }
        let fen_line = extract_fields(line);
        let hash = fen_hash(&fen_line.fen, zobrist_table);
        v.push(FenLineEnriched { fen_line, hash });
    }
    v
}

fn fen_hash(fen: &str, zobrist_table: &zobrist::Zobrist) -> zobrist::ZobristHash {
    let position = fen::Fen::decode(fen).unwrap();
    let bit_position = bitboard::BitPosition::from(position);
    zobrist::ZobristHash::zobrist_partial_hash_from_position(&bit_position, zobrist_table)
}

fn known_mat_to_map(fen_lines_enriched: &[FenLineEnriched]) -> FenLineMap {
    let zobrist_table = zobrist::Zobrist::new();
    let mut m: FenLineMap = HashMap::new();
    for fen_line_enriched in fen_lines_enriched
        .iter()
        .filter(|line| !line.fen_line.mat_in.is_empty())
    {
        let hash = fen_hash(&fen_line_enriched.fen_line.fen, &zobrist_table);
        m.insert(hash, fen_line_enriched.clone());
    }
    m
}

fn fens_to_map(
    fens: &[String],
    zobrist_table: &zobrist::Zobrist,
) -> HashMap<zobrist::ZobristHash, String> {
    let mut m: HashMap<zobrist::ZobristHash, String> = HashMap::new();
    for fen in fens {
        let hash = fen_hash(fen, zobrist_table);
        m.insert(hash, fen.to_string());
    }
    m
}

fn build_possible_and_valid_positions(
    fen: &str,
    fen_lines_enriched: &FenLineMap,
    mat_in_target: u8,
    zobrist_table: &zobrist::Zobrist,
) -> Vec<String> {
    let fen_possible_positions = build_fen_possible_positions(fen);
    let m_fen_possible_positions: HashMap<zobrist::ZobristHash, String> =
        fens_to_map(&fen_possible_positions, zobrist_table);
    intersect_with_reader(fen_lines_enriched, &m_fen_possible_positions, mat_in_target)
}

async fn get_lines_to_update_mat_in_n(
    fen_lines_enriched: &FenLineMap,
    known_positions: &FenLineMap,
    engine_player1: &EngineMat,
    self_actor: actix::Addr<dispatcher::EngineDispatcher>,
    mat_in_target: u8,
    zobrist_table: &zobrist::Zobrist,
) -> FenLineMap {
    let mut modified_lines: FenLineMap = HashMap::new();
    // loop over each line
    for (i, fen_line_enriched) in fen_lines_enriched.values().enumerate() {
        // extract fields
        let FenLine {
            fen,
            mat_in,
            move_str: _,
        } = fen_line_enriched.fen_line.clone();
        if let Ok(mat_in_value) = mat_in.parse::<u8>() {
            if mat_in_target - 1 == mat_in_value {
                let fen_possible_positions = build_possible_and_valid_positions(
                    &fen,
                    fen_lines_enriched,
                    mat_in_target,
                    zobrist_table,
                );
                let end_position = fen::Fen::decode(&fen).expect("Failed to decode FEN");
                for start_fen in fen_possible_positions {
                    let start_position =
                        fen::Fen::decode(&start_fen).expect("Failed to decode FEN");
                    if let Some((_mat_in, bitboard_move)) = try_reach_position(
                        1,
                        mat_in_target,
                        &start_position,
                        Some(&end_position),
                        engine_player1,
                        known_positions,
                        self_actor.clone(),
                        zobrist_table,
                    )
                    .await
                    {
                        let hash = fen_hash(&start_fen, zobrist_table);
                        let move_str =
                            LongAlgebricNotationMove::build_from_b_move(bitboard_move).cast();
                        let new_fen_line_enriched = FenLineEnriched {
                            fen_line: FenLine {
                                fen: start_fen.to_string(),
                                mat_in: mat_in_target.to_string(),
                                move_str,
                            },
                            hash,
                        };
                        if let Some(fen_line_enriched) =
                            modified_lines.get_mut(&new_fen_line_enriched.hash)
                        {
                            if fen_line_enriched.fen_line.mat_in
                                == new_fen_line_enriched.fen_line.mat_in
                                && !fen_line_enriched
                                    .fen_line
                                    .move_str
                                    .contains(&new_fen_line_enriched.fen_line.move_str)
                            {
                                fen_line_enriched.fen_line.move_str = format!(
                                    "{},{}",
                                    fen_line_enriched.fen_line.move_str,
                                    new_fen_line_enriched.fen_line.move_str
                                );
                            }
                        } else {
                            modified_lines
                                .insert(new_fen_line_enriched.hash.clone(), new_fen_line_enriched);
                        }
                    }
                }
            }
        }
        if modified_lines.len() > 1 {
            //break;
        }
        if i % 1000 == 0 {
            tracing::info!(
                "Lines to update: {}. Processed {} lines...",
                modified_lines.len(),
                i
            );
        }
    }
    modified_lines
}

fn read_lines<R>(reader: R) -> std::io::Result<Vec<FenLineEnriched>>
where
    R: BufRead,
{
    let lines: Vec<String> = reader.lines().collect::<std::io::Result<Vec<_>>>()?;

    let zobrist_table = zobrist::Zobrist::new();
    let fen_lines = to_fen_lines_enriched(&lines, &zobrist_table);

    Ok(fen_lines)
}

fn intersect_filter(
    key: &zobrist::ZobristHash,
    m_fen_lines_enriched: &FenLineMap,
    mat_in_target: u8,
) -> Option<String> {
    let mut maybe_fen: Option<String> = None;
    if let Some(fen_lines_enriched) = m_fen_lines_enriched.get(key) {
        let FenLine {
            fen,
            mat_in,
            move_str: _,
        } = fen_lines_enriched.fen_line.clone();
        if mat_in.is_empty() || mat_in.parse::<u8>().unwrap() == mat_in_target {
            maybe_fen = Some(fen);
        }
    }
    maybe_fen
}

fn intersect_with_reader(
    fen_lines_enriched: &FenLineMap,
    m_fen_possible_positions: &HashMap<zobrist::ZobristHash, String>,
    mat_in_target: u8,
) -> Vec<String> {
    let intersected_lines = m_fen_possible_positions
        .keys()
        .filter_map(|key| intersect_filter(key, fen_lines_enriched, mat_in_target))
        .collect();
    intersected_lines
}

fn write_modified_lines_mat_in_n<W: Write>(
    reader: impl BufRead,
    writer: &mut W,
    modified_lines: &FenLineMap,
) -> std::io::Result<()> {
    let zobrist_table = zobrist::Zobrist::new();
    let mut lines_updated = 0;

    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            continue;
        }

        let line = line?;
        let extracted_line = extract_fields(&line);
        let hash = fen_hash(&extracted_line.fen, &zobrist_table);

        if let Some(fen_line_enriched) = modified_lines.get(&hash) {
            lines_updated += 1;
            writeln!(writer, "{}", fen_line_enriched.fen_line)?;
        } else {
            writeln!(writer, "{line}")?;
        }
    }

    assert_eq!(
        lines_updated,
        modified_lines.len(),
        "Lines updated: {}, Modified lines: {}",
        lines_updated,
        modified_lines.len()
    );

    writer.flush()?;
    Ok(())
}

// build all possible position having a distance of 1 for white and black bitboard
fn build_fen_possible_positions(fen: &str) -> Vec<String> {
    let position: fen::Position = fen::Fen::decode(fen).unwrap();
    let chessboard = position.chessboard();
    let bitboards = &BitBoardsWhiteAndBlack::from(*chessboard);
    build_bitboards_distance1(bitboards)
}

fn moves2bitboard_moves(
    color: square::Color,
    moves: Vec<piece_move::PieceMoves>,
    bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
) -> Vec<bitboard::BitBoardMove> {
    let mut bitboard_moves: Vec<bitboard::BitBoardMove> = vec![];
    for piece_moves in &moves {
        for to in piece_moves.moves().iter() {
            let bitboard_move = bitboard::BitBoardMove::from(
                color,
                piece_moves.type_piece(),
                piece_moves.index(),
                to,
                bit_boards_white_and_black,
            );
            bitboard_moves.extend(bitboard_move);
        }
    }
    bitboard_moves
}

fn build_bitboards_distance1(bitboards: &bitboard::BitBoardsWhiteAndBlack) -> Vec<String> {
    let mut fens: Vec<String> = Vec::new();
    let mut bit_position_status = bitboard::BitPositionStatus::default();
    // useless to control check
    let piece_moves_white: Vec<piece_move::PieceMoves> =
        bitboards.gen_moves_no_check(&square::Color::White, None, &bit_position_status);
    let moves_white: Vec<bitboard::BitBoardMove> =
        moves2bitboard_moves(square::Color::White, piece_moves_white, bitboards);
    bit_position_status.set_player_turn_white(false);
    // useless to control check
    let piece_moves_black: Vec<piece_move::PieceMoves> =
        bitboards.gen_moves_no_check(&square::Color::Black, None, &bit_position_status);
    let moves_black: Vec<bitboard::BitBoardMove> =
        moves2bitboard_moves(square::Color::Black, piece_moves_black, bitboards);
    // check pieces are on different squares
    for move_white in moves_white {
        for move_black in &moves_black {
            if move_white.end() != move_black.end() {
                let mut new_bitboards = bitboards.clone();
                new_bitboards.move_piece(&move_white, &mut zobrist::ZobristHash::default(), None);
                new_bitboards.move_piece(move_black, &mut zobrist::ZobristHash::default(), None);
                let status = PositionStatus::default();
                let chessboard = new_bitboards.to();
                let position: Position = Position::build(chessboard, status);
                let fen = fen::Fen::encode(&position).unwrap();
                fens.push(fen);
            }
        }
    }
    fens
}

async fn mat_in_one(
    input_file: &str,
    output_file: &str,
    engine_player1: &EngineMat,
    self_actor: actix::Addr<dispatcher::EngineDispatcher>,
    zobrist_table: &zobrist::Zobrist,
) -> std::io::Result<()> {
    init_update_file(
        input_file,
        output_file,
        engine_player1,
        self_actor,
        zobrist_table,
    )
    .await
}

fn to_fen_lines_map(fen_lines_enriched: &[FenLineEnriched]) -> FenLineMap {
    let mut fen_lines_map: FenLineMap = HashMap::new();
    for fen_line_enriched in fen_lines_enriched {
        fen_lines_map.insert(fen_line_enriched.hash.clone(), fen_line_enriched.clone());
    }
    fen_lines_map
}

async fn mat_in_n<R>(
    reader: R,
    mat_in: u8,
    engine_player1: &EngineMat,
    self_actor: actix::Addr<dispatcher::EngineDispatcher>,
    zobrist_table: &zobrist::Zobrist,
) -> std::io::Result<FenLineMap>
where
    R: BufRead,
{
    let fen_lines_enriched = read_lines(reader)?;
    let m_fen_lines_enriched = to_fen_lines_map(&fen_lines_enriched);

    // put apart known mat
    let known_mat_m: FenLineMap = known_mat_to_map(&fen_lines_enriched);

    tracing::info!("Updating file with mat in {} positions...", mat_in);
    let modified_lines: FenLineMap = get_lines_to_update_mat_in_n(
        &m_fen_lines_enriched,
        &known_mat_m,
        engine_player1,
        self_actor,
        mat_in,
        zobrist_table,
    )
    .await;
    tracing::info!("Lines to be updated: {}", modified_lines.len());
    Ok(modified_lines)
}

#[actix::main]
async fn main() {
    init_trace();
    let args: Vec<String> = env::args().collect();
    let mat_in = if args.len() > 1 {
        args[1].parse::<u8>().unwrap_or(1)
    } else {
        1
    };

    let max_depth = 1;
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
    let engine_player1 = engine_mat::EngineMat::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        &config_params::MatConfig::new(max_depth),
    );
    let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
        Arc::new(engine_player1.clone()),
        debug_actor_opt.clone(),
        None,
    );
    let self_actor = engine_player1_dispatcher.start();
    let zobrist_table = zobrist::Zobrist::new();
    let output_file = "krk_position_updated.csv";

    let result = if mat_in > 1 {
        // backup
        let backup_file = backup_filename(output_file);
        tracing::info!("backup file, {} to {}", output_file, backup_file);
        copy_file(output_file, &backup_file).expect("Error creating backup file");
        // reader and writer
        let in_file = File::open(backup_file.clone())
            .unwrap_or_else(|_| panic!("Cannot open file {}", backup_file));
        let reader = BufReader::new(in_file);
        let out_file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_file)
            .unwrap_or_else(|_| panic!("Cannot open file {}", output_file));
        let mut writer = BufWriter::new(out_file);
        let modified_lines = mat_in_n(reader, mat_in, &engine_player1, self_actor, &zobrist_table)
            .await
            .unwrap_or_else(|_| panic!("Error when reading file {}", output_file));
        let in_file = File::open(backup_file.clone())
            .unwrap_or_else(|_| panic!("Cannot open file {}", backup_file));
        let reader = BufReader::new(in_file);
        write_modified_lines_mat_in_n(reader, &mut writer, &modified_lines)
    } else {
        mat_in_one(
            "krk_position.csv",
            output_file,
            &engine_player1,
            self_actor,
            &zobrist_table,
        )
        .await
    };
    if let Err(e) = result {
        tracing::error!("Error updating file: {}", e);
    } else {
        tracing::info!("File updated successfully.");
    }
}

#[cfg(test)]
use std::io::Cursor;

#[actix::test]
async fn test_update_line() {
    let zobrist_table = zobrist::Zobrist::new();
    let engine_player1 =
        engine_mat::EngineMat::new(None, zobrist_table, &config_params::MatConfig::new(1));
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
        Arc::new(engine_player1.clone()),
        debug_actor_opt.clone(),
        None,
    );
    let self_actor = engine_player1_dispatcher.start();

    let line = "6k1/8/6K1/8/8/8/8/R7 w - - 0 1";
    let updated = update_line_mat_in_one(
        line,
        &engine_player1,
        self_actor,
        1,
        &zobrist::Zobrist::new(),
    )
    .await;
    assert_eq!(updated, "6k1/8/6K1/8/8/8/8/R7 w - - 0 1;1;a1a8");
}

#[actix::test]
async fn test_position_reached() {
    //let fen = "6k1/R7/6K1/8/8/8/8/8 w - - 0 1";
    //let position = fen::Fen::decode(fen).unwrap();
    let zobrist_table = zobrist::Zobrist::new();
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
    let engine_player1 = engine_mat::EngineMat::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        &config_params::MatConfig::new(1),
    );
    let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
        Arc::new(engine_player1.clone()),
        debug_actor_opt.clone(),
        None,
    );
    let self_actor = engine_player1_dispatcher.start();

    let content: Vec<String> = vec![
        "fen;mat_in;move",
        "6k1/R7/6K1/8/8/8/8/8 w - - 0 0;1;a7a8",
        "7k/1R6/6K1/8/8/8/8/8 w - - 0 0;;",
    ]
    .into_iter()
    .map(|line| line.to_string())
    .collect();
    let fen_lines_to_update = to_fen_lines_enriched(&content, &zobrist_table);
    let m_fen_lines_to_update = to_fen_lines_map(&fen_lines_to_update);
    let known_mat_m: FenLineMap = known_mat_to_map(&fen_lines_to_update);
    let lines_to_update = get_lines_to_update_mat_in_n(
        &m_fen_lines_to_update,
        &known_mat_m,
        &engine_player1,
        self_actor,
        2,
        &zobrist_table,
    )
    .await;
    assert!(!lines_to_update.is_empty())
}

#[actix::test]
async fn test_find_white_move() {
    let fen_start = "6k1/R7/6K1/8/8/8/8/8 w - - 0 1";
    let fen_end = "7k/1R6/6K1/8/8/8/8/8 w - - 0 1";
    let start_position = fen::Fen::decode(fen_start).expect("Failed to decode FEN");
    let end_position = fen::Fen::decode(fen_end).expect("Failed to decode FEN");
    let b_move = find_white_move(&start_position, &end_position);
    let expected = bitboard::BitBoardMove::new(
        square::Color::White,
        square::TypePiece::Rook,
        bitboard::BitIndex::new(48),
        bitboard::BitIndex::new(49),
        None,
        None,
    );
    assert_eq!(b_move, expected);
}

#[actix::test]
async fn test_update_mat_in_n() {
    let lines: Vec<String> = vec![
        "fen;mat_in;move",
        "6k1/R7/6K1/8/8/8/8/8 w - - 0 0;1;a7a8",
        "7k/1R6/6K1/8/8/8/8/8 w - - 0 0;;",
    ]
    .into_iter()
    .map(|line| line.to_string())
    .collect();
    let input = Cursor::new(lines.join("\n"));
    let mut output = Vec::new();
    let mat_in = 2;

    let zobrist_table = zobrist::Zobrist::new();
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
    let engine_player1 = engine_mat::EngineMat::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        &config_params::MatConfig::new(1),
    );
    let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
        Arc::new(engine_player1.clone()),
        debug_actor_opt.clone(),
        None,
    );
    let self_actor = engine_player1_dispatcher.start();

    let fen_lines_enriched: Vec<FenLineEnriched> = to_fen_lines_enriched(&lines, &zobrist_table);
    let m_fen_lines_enriched = to_fen_lines_map(&fen_lines_enriched);
    let fen_mat_in_1 = "6k1/R7/6K1/8/8/8/8/8 w - - 0 0";
    let hash_mat_in_one = fen_hash(fen_mat_in_1, &zobrist::Zobrist::new());
    let mut known_mat_m: FenLineMap = HashMap::new();
    println!("hash fen mat in one: {:?}", hash_mat_in_one.clone());
    known_mat_m.insert(
        hash_mat_in_one.clone(),
        FenLineEnriched {
            fen_line: FenLine {
                fen: fen_mat_in_1.to_string(),
                mat_in: "1".to_string(),
                move_str: "a7a8".to_string(),
            },
            hash: hash_mat_in_one,
        },
    );

    let modified_lines = get_lines_to_update_mat_in_n(
        &m_fen_lines_enriched,
        &known_mat_m,
        &engine_player1,
        self_actor,
        mat_in,
        &zobrist_table,
    )
    .await;

    write_modified_lines_mat_in_n(input, &mut output, &modified_lines)
        .expect("Failed to write modified lines");

    let expected: Vec<String> = vec![
        "6k1/R7/6K1/8/8/8/8/8 w - - 0 0;1;a7a8",
        "7k/1R6/6K1/8/8/8/8/8 w - - 0 0;2;b7a7",
        "",
    ]
    .into_iter()
    .map(|line| line.to_string())
    .collect();
    let expected = expected.join("\n");

    let result = String::from_utf8(output).unwrap();
    assert_eq!(result, expected);
}

#[actix::test]
async fn test_multiple_equivalent_move_mat_in_n() {
    let zobrist_table = zobrist::Zobrist::new();
    let lines: Vec<String> = vec![
        "fen;mat_in;move",
        "6R1/8/8/8/8/8/5K1k/8 w - - 0 0;1;g8h8",
        "8/6R1/8/8/8/8/5K1k/8 w - - 0 0;1;g7h7",
        "8/8/6R1/8/8/8/5K1k/8 w - - 0 0;1;g6h6",
        "8/8/8/6R1/8/8/5K1k/8 w - - 0 0;1;g5h5",
        "8/8/8/8/6R1/8/5K1k/8 w - - 0 0;1;g4h4",
    ]
    .into_iter()
    .map(|line| line.to_string())
    .collect();

    let fen = "8/8/8/8/8/8/5KR1/7k w - - 0 0";
    let fen_lines_enriched: Vec<FenLineEnriched> = to_fen_lines_enriched(&lines, &zobrist_table);
    let m_fen_lines_enriched = to_fen_lines_map(&fen_lines_enriched);

    let fen_possible_positions =
        build_possible_and_valid_positions(fen, &m_fen_lines_enriched, 1, &zobrist_table);
    assert_eq!(fen_possible_positions.len(), lines.len() - 1);
}

#[actix::test]
async fn test_update_multiple_equivalent_move_mat_in_n() {
    let lines: Vec<String> = vec![
        "fen;mat_in;move",
        "6R1/8/8/8/8/8/5K1k/8 w - - 0 0;1;g8h8",
        "8/6R1/8/8/8/8/5K1k/8 w - - 0 0;1;g7h7",
        "8/8/6R1/8/8/8/5K1k/8 w - - 0 0;1;g6h6",
        "8/8/8/6R1/8/8/5K1k/8 w - - 0 0;1;g5h5",
        "8/8/8/8/6R1/8/5K1k/8 w - - 0 0;1;g4h4",
        "8/8/8/8/8/8/5KR1/7k w - - 0 0;;",
    ]
    .into_iter()
    .map(|line| line.to_string())
    .collect();
    let input = Cursor::new(lines.join("\n"));
    let mat_in = 2;

    let zobrist_table = zobrist::Zobrist::new();
    let debug_actor_opt: Option<debug::DebugActor> = None;
    let game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
    let engine_player1 = engine_mat::EngineMat::new(
        debug_actor_opt.clone(),
        game_manager.zobrist_table(),
        &config_params::MatConfig::new(1),
    );
    let engine_player1_dispatcher = dispatcher::EngineDispatcher::new(
        Arc::new(engine_player1.clone()),
        debug_actor_opt.clone(),
        None,
    );
    let self_actor = engine_player1_dispatcher.start();
    let modified_lines = mat_in_n(input, mat_in, &engine_player1, self_actor, &zobrist_table)
        .await
        .unwrap();
    assert!(modified_lines.len() == 1);
    let moves = &modified_lines.values().next().unwrap().fen_line.move_str;
    let mut moves = moves.split(",").collect::<Vec<&str>>();
    moves.sort();
    let expected_moves: Vec<&str> = vec!["g2g4", "g2g5", "g2g6", "g2g7", "g2g8"];
    assert_eq!(moves, expected_moves);
}
