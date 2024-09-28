pub mod configuration;
pub mod engine;
pub mod parameters;
pub mod player;

use crate::board::bitboard::piece_move::{CheckStatus, GenMoves};
use crate::board::bitboard::{zobrist, BitBoardMove, BitBoardsWhiteAndBlack};
use crate::board::{bitboard, fen, square};
use crate::uci::notation::{self, LongAlgebricNotationMove};
use actix::prelude::*;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EndGame {
    #[default]
    None,
    Pat,
    Mat,
    NoPawnAndCapturex50,   // 50 moves rule
    InsufficientMaterial,  // King (+ bishop or knight) vs King (+ bishop or knight)
    Repetition3x,          // 3x the same position
    TimeOutCannotCheckMat, // Timeout but only a King, King + Bishop or Knight
    NullAgreement,         // Two players agree to end the game
}

#[derive(Message)]
#[rtype(result = "Option<LongAlgebricNotationMove>")]
pub struct GetBestMove;

impl<T: engine::EngineActor> Handler<GetBestMove> for Game<T> {
    type Result = Option<LongAlgebricNotationMove>;

    fn handle(&mut self, _msg: GetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        self.best_move
    }
}

#[derive(Message)]
#[rtype(result = "Result<History, ()>")]
pub struct GetHistory;

impl<T: engine::EngineActor> Handler<GetHistory> for Game<T> {
    type Result = Result<History, ()>;

    fn handle(&mut self, _msg: GetHistory, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.history().clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<EndGame, ()>")]
pub struct GetEndGame;

impl<T: engine::EngineActor> Handler<GetEndGame> for Game<T> {
    type Result = Result<EndGame, ()>;

    fn handle(&mut self, _msg: GetEndGame, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.end_game().clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<configuration::Configuration, ()>")]
pub struct GetConfiguration;

impl<T: engine::EngineActor> Handler<GetConfiguration> for Game<T> {
    type Result = Result<configuration::Configuration, ()>;

    fn handle(&mut self, _msg: GetConfiguration, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.configuration().clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct PlayMoves(pub Vec<LongAlgebricNotationMove>);

impl<T: engine::EngineActor> Handler<PlayMoves> for Game<T> {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PlayMoves, _ctx: &mut Self::Context) -> Self::Result {
        self.play_moves(msg.0)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), String>")]
pub enum UciCommand {
    InitPosition,
    UpdatePosition(String, fen::Position),
    DepthFinite(u32),
    TimePerMoveInMs(u32),
    SearchInfinite,
    Wtime(u64),
    Btime(u64),
    SearchMoves(Vec<notation::LongAlgebricNotationMove>),
    ValidMoves(Vec<notation::LongAlgebricNotationMove>),
    StartEngine,
    StopEngine,
}

pub type GameActor<T> = Addr<Game<T>>;

#[derive(Debug, Default, Clone)]
pub struct History {
    fen: String,
    moves: Vec<BitBoardMove>,
}
impl History {
    pub fn init(&mut self) {
        self.set_fen(fen::FEN_START_POSITION);
    }
    pub fn set_fen(&mut self, fen: &str) {
        self.fen = fen.to_string();
        self.moves = vec![];
    }
    pub fn add_moves(&mut self, m: BitBoardMove) {
        self.moves.push(m);
    }
}

// Actor definition
#[derive(Default)]
pub struct Game<T: engine::EngineActor> {
    configuration: configuration::Configuration,
    best_move: Option<LongAlgebricNotationMove>,
    history: History,
    // once a move is played, we update moves for the next player
    moves: Vec<BitBoardMove>,
    hash_positions: zobrist::ZobristHistory,
    end_game: EndGame,
    zobrist_table: zobrist::Zobrist,
    players: player::Players<T>,
}

impl<T: engine::EngineActor> Game<T> {
    pub fn new() -> Self {
        let mut game = Game::default();
        game.zobrist_table = game.zobrist_table.init();
        // init moves and game status
        game.update_moves();
        game
    }
    fn add_hash(&mut self, hash: zobrist::ZobristHash) {
        self.hash_positions.push(hash);
    }
    // build the hash table
    fn init_hash_table(&mut self) {
        let position = self
            .configuration
            .opt_position()
            .expect("Internal error: cannot find a position after init");
        let bit_position = bitboard::BitPosition::from(position);
        // reset hash from new position
        self.hash_positions = zobrist::ZobristHistory::default();
        let hash =
            zobrist::ZobristHash::zobrist_hash_from_position(&bit_position, &self.zobrist_table);
        self.add_hash(hash);
    }
    pub fn last_hash(&self) -> zobrist::ZobristHash {
        self.hash_positions
            .list()
            .last()
            .expect("Internal error: No hash position computed")
            .clone()
    }
    pub fn configuration(&self) -> &configuration::Configuration {
        &self.configuration
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub fn end_game(&self) -> EndGame {
        self.end_game.clone()
    }

    pub fn update_moves(&mut self) {
        if let Some(position) = self.configuration.opt_position() {
            let bitboard_position = bitboard::BitPosition::from(position);
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
            let check_status = bit_boards_white_and_black.check_status(&color);
            let capture_en_passant = bit_position_status.pawn_en_passant();
            self.moves = bit_boards_white_and_black.gen_moves_for_all(
                &color,
                check_status,
                &capture_en_passant,
                bit_position_status,
            );
            if self.moves.is_empty() {
                match check_status {
                    CheckStatus::None => self.end_game = EndGame::Pat,
                    _ => self.end_game = EndGame::Mat,
                }
            } else if bit_position_status.n_half_moves() >= 100 {
                self.end_game = EndGame::NoPawnAndCapturex50
            } else if self.check_insufficient_material(bit_boards_white_and_black) {
                self.end_game = EndGame::InsufficientMaterial
            } else if self
                .hash_positions
                .check_3x(bit_position_status.n_half_moves())
            {
                self.end_game = EndGame::Repetition3x
            }
        }
    }

    fn check_insufficient_material(
        &self,
        bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
    ) -> bool {
        let bitboard_white = bit_boards_white_and_black.bit_board_white();
        let bitboard_black = bit_boards_white_and_black.bit_board_black();
        let white_relevant_pieces = *bitboard_white.rooks().bitboard()
            | *bitboard_white.queens().bitboard()
            | *bitboard_white.pawns().bitboard();
        let black_relevant_pieces = *bitboard_black.rooks().bitboard()
            | *bitboard_black.queens().bitboard()
            | *bitboard_black.pawns().bitboard();
        if white_relevant_pieces.empty() && black_relevant_pieces.empty() {
            let white_other =
                *bitboard_white.bishops().bitboard() | *bitboard_white.knights().bitboard();
            let black_other =
                *bitboard_black.bishops().bitboard() | *bitboard_black.knights().bitboard();
            white_other.one_bit_set_max() && black_other.empty()
                || white_other.empty() && black_other.one_bit_set_max()
        } else {
            false
        }
    }

    #[allow(dead_code)]
    fn show(&self) {
        println!(
            "{}",
            self.configuration.opt_position().unwrap().chessboard()
        );
    }
    pub fn play_moves(&mut self, valid_moves: Vec<LongAlgebricNotationMove>) -> Result<(), String> {
        let mut result: Result<(), String> = Ok(());
        if let Some(position) = self.configuration.opt_position() {
            let mut bit_position = bitboard::BitPosition::from(position);
            for m in valid_moves {
                let color = bit_position.bit_position_status().player_turn();
                match check_move(color, m, &bit_position) {
                    Err(err) => {
                        result = Err(err);
                        break;
                    }
                    Ok(b_move) => {
                        println!("play: {:?}", b_move);
                        let mut hash = self.last_hash();
                        bit_position =
                            bit_position.move_piece(&b_move, &mut hash, &self.zobrist_table);
                        // update hash history
                        self.add_hash(hash);
                        self.configuration.update_position(bit_position.to());
                        self.history.add_moves(b_move);
                        self.show();
                        self.update_moves();
                    }
                }
            }
        } else {
            result = Err("moves ignored since no position has been defined".to_string());
        }
        result
    }

    pub fn get_players(&self) -> &player::Players<T> {
        &self.players
    }
    pub fn set_players(&mut self, players: player::Players<T>) {
        self.players = players;
    }
}
impl<T: engine::EngineActor> Actor for Game<T> {
    type Context = Context<Self>;
}

impl<T: engine::EngineActor> Handler<UciCommand> for Game<T> {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UciCommand, _ctx: &mut Self::Context) -> Self::Result {
        let mut result = Ok(());
        match msg {
            UciCommand::Btime(time) => {
                let mut params = self.configuration().parameters().clone();
                params.set_btime(time);
                self.configuration.update_parameters(params);
            }
            UciCommand::InitPosition => {
                let position = fen::Position::build_initial_position();
                self.configuration.update_position(position);
                self.init_hash_table();
                self.history.init();
            }
            UciCommand::Wtime(time) => {
                let mut params = self.configuration().parameters().clone();
                params.set_wtime(time);
                self.configuration.update_parameters(params);
            }
            UciCommand::DepthFinite(depth) => {
                let mut params = self.configuration().parameters().clone();
                params.set_depth(depth);
                self.configuration.update_parameters(params);
            }
            UciCommand::SearchInfinite => {
                let mut params = self.configuration().parameters().clone();
                params.set_depth_infinite();
                self.configuration.update_parameters(params);
            }
            UciCommand::TimePerMoveInMs(time) => {
                let mut params = self.configuration().parameters().clone();
                params.set_time_per_move_in_ms(time);
                self.configuration.update_parameters(params);
            }
            UciCommand::UpdatePosition(fen, position) => {
                self.configuration.update_position(position);
                self.init_hash_table();
                self.update_moves();
                self.history.set_fen(&fen);
            }
            UciCommand::SearchMoves(search_moves) => {
                let mut params = self.configuration().parameters().clone();
                params.set_search_moves(search_moves);
                self.configuration.update_parameters(params);
            }
            UciCommand::ValidMoves(valid_moves) => {
                result = self.play_moves(valid_moves);
            }
            UciCommand::StartEngine => {
                if let Some(position) = self.configuration().opt_position() {
                    let engine_actor_or_error =
                        self.players.get_engine(position.status().player_turn());
                    match engine_actor_or_error {
                        Ok(engine) => {
                            engine.go();
                        }
                        Err(err) => result = Err(err),
                    }
                }
            }
            UciCommand::StopEngine => match self.configuration.opt_position() {
                None => {
                    self.best_move = None;
                    result =
                        Err("No bestmove since no valid position has been entered.".to_string());
                }
                Some(_) => {
                    // TODO: get bestmove from engine
                    self.best_move =
                        Some(notation::LongAlgebricNotationMove::build_from_str("e2e4").unwrap());
                } // Stop engine search
            },
        }
        result
    }
}

// The start square must contain a piece
fn check_move(
    player_turn: square::Color,
    m: notation::LongAlgebricNotationMove,
    bitboard_position: &bitboard::BitPosition,
) -> Result<BitBoardMove, String> {
    let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
    let start_square = bit_boards_white_and_black.peek(m.start());
    let end_square = bit_boards_white_and_black.peek(m.end());
    match (start_square, end_square) {
        (square::Square::Empty, _) => Err(format!("empty start square {}", m.start().value())),
        (square::Square::NonEmpty(piece), square::Square::Empty) => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion());
            check_move_level2(b_move, bitboard_position)
        },
        (square::Square::NonEmpty(piece), square::Square::NonEmpty(capture)) if capture.color() != piece.color() => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), Some(capture.type_piece()), m.opt_promotion());
            check_move_level2(b_move, bitboard_position)
        },
        (square::Square::NonEmpty(_), square::Square::NonEmpty(_)) => Err(format!("Invalid move from {} to {} since the destination square contains a piece of the same color as the piece played." , m.start().value(), m.end().value())),
    }
}

fn check_move_level2(
    b_move: BitBoardMove,
    bitboard_position: &bitboard::BitPosition,
) -> Result<BitBoardMove, String> {
    let color = bitboard_position.bit_position_status().player_turn();
    let bit_position_status = bitboard_position.bit_position_status();
    let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
    let check_status = bit_boards_white_and_black.check_status(&color);
    let capture_en_passant = bit_position_status.pawn_en_passant();
    let moves = bit_boards_white_and_black.gen_moves_for_all(
        &color,
        check_status,
        &capture_en_passant,
        bit_position_status,
    );
    if moves.iter().any(|m| *m == b_move) {
        Ok(b_move)
    } else {
        let possible_moves_for_piece: Vec<String> = moves
            .iter()
            .filter(|m| m.start() == b_move.start())
            .map(|m| {
                notation::LongAlgebricNotationMove::new(m.start(), m.end(), m.promotion()).cast()
            })
            .collect();
        let invalid_move = notation::LongAlgebricNotationMove::new(
            b_move.start(),
            b_move.end(),
            b_move.promotion(),
        )
        .cast();
        Err(format!(
            "The move {} is invalid. Valid moves for this piece are: {:?}",
            invalid_move, possible_moves_for_piece
        ))
    }
}

#[cfg(test)]
mod tests {
    use actix::Actor;

    use crate::{game, uci};

    use super::{configuration, engine};

    async fn get_configuration<T: engine::EngineActor>(
        game_actor: &game::GameActor<T>,
    ) -> configuration::Configuration {
        let result = game_actor.send(game::GetConfiguration).await.unwrap();
        result.unwrap()
    }

    #[actix::test]
    async fn test_game_capture_en_passant() {
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 e7e5 d5e6", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        // unwrap() is the test
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
    }
    #[actix::test]
    async fn test_game_pawn_move_invalid() {
        let inputs = vec!["position startpos moves e2e4 e7e5 e4e5", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let r = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(r.is_err());
    }

    #[actix::test]
    async fn test_game_mat() {
        let inputs = vec![
            "position startpos moves e2e4 e7e5 f1c4 a7a6 d1f3 a6a5 f3f7",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Mat)
    }
    #[actix::test]
    async fn test_game_pat_white_first() {
        let inputs = vec![
            "position fen k7/7R/1R6/8/8/8/8/7K w - - 0 1 moves h1g1",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_pat_black_first() {
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K b - - 0 1", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_weird() {
        let inputs = vec!["position startpos moves d2d4 d7d5 b1c3 a7a6 c1f4 a6a5 d1d2 a5a4 e1c1 a4a3 h2h3 a3b2 c1b1 a8a2 h3h4 a2a1 b1b2", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_ok())
    }
    #[actix::test]
    async fn test_game_blocked_pawn_ckeck() {
        let inputs = vec!["position startpos moves e2e4 e7e5 a2a3 d8h4 f2f3", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_err())
    }
    #[actix::test]
    async fn test_game_block_ckeck() {
        let inputs = vec![
            "position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1f3",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_err())
    }
    #[actix::test]
    async fn test_game_block_ckeck2() {
        let inputs = vec![
            "position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1e2",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_ok())
    }
    #[actix::test]
    async fn test_game_escape() {
        let inputs = vec![
            "position startpos moves e2e4 c7c5 f1c4 d7d6 d1h5 a7a6 h5f7 e8d7",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_ok())
    }
    #[actix::test]
    async fn test_game_king_close_to_king() {
        let inputs = vec![
            "position fen r7/8/8/4k3/8/4K3/8/7R w - - 0 1 moves e3e4",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_err())
    }
    #[actix::test]
    async fn test_game_rule_insufficient_material() {
        let inputs = vec!["position fen k7/8/8/8/8/8/8/7K b - - 0 1", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::InsufficientMaterial)
    }
    #[actix::test]
    async fn test_game_rule_50xmoves() {
        // f5e5 -> forbidden => review attackers() ?
        let seq1x10 = "h1g1 a8b8 g1f1 b8c8 f1e1 c8d8 e1d1 d8e8 d1c1 e8f8 c1b1 f8g8 b1b2 g8g7 b2c2 g7f7 c2d2 f7e7 d2e2 e7d7";
        let seq2x10 = "e2f2 d7c7 f2g2 c7b7 g2h2 b7a7 h3h4 a6a5 h2h3 a7a6 h3g3 a6b6 g3f3 b6c6 f3e3 c6d6 e3d3 d6e6 d3c3 e6f6";
        let seq3x10 = "c3b3 f6g6 b3b4 g6g5 b4c4 g5f5 c4d4 f5e6 d4e4 e6d6 e4d4 d6c6 d4c4 c6b6 c4b4 b6a6 b4b3 a6a7 b3b2 a7b7";
        let seq4x10 = "b2c2 b7c7 c2d2 c7d7 d2e2 d7e7 e2f2 e7f7 f2g2 f7g7 g2f2 g7g8 f2e2 g8f8 e2d2 f8e8 d2c2 e8d8 c2b2 d8c8";
        let seq5x10 = "b2b1 c8b8 b1c1 b8a8 c1d1 a8a7 d1e1 a7b7 e1f1 b7c7 f1g1 c7d7 g1h1 d7e7 h1h2 e7e6 h2g2 e6e5 g2f2 e5f5";
        let movesx50 = format!(
            "{} {} {} {} {}",
            seq1x10, seq2x10, seq3x10, seq4x10, seq5x10
        );
        let fen = format!(
            "position fen k7/8/r7/8/8/7R/8/7K w - - 0 1 moves {}",
            movesx50
        );
        let inputs = vec![&fen, "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::NoPawnAndCapturex50)
    }

    #[actix::test]
    async fn test_game_3x_position() {
        let moves = "h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!("position fen k7/8/r7/8/8/7R/8/7K w - - 0 1 moves {}", moves);
        let inputs = vec![&fen, "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Repetition3x)
    }
    #[actix::test]
    async fn test_game_3x_position_with_pawn() {
        let moves = "h7h6 h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!(
            "position fen k7/7p/r7/8/8/7R/8/7K b - - 0 1 moves {}",
            moves
        );
        let inputs = vec![&fen, "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::<engine::EngineDummy>::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Repetition3x)
    }
}
