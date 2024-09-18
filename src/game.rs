pub mod configuration;
pub mod parameters;

use crate::board::bitboard::piece_move::GenMoves;
use crate::board::bitboard::BitBoardMove;
use crate::board::{bitboard, fen, square};
use crate::uci::notation::{self, LongAlgebricNotationMove};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "Option<LongAlgebricNotationMove>")]
pub struct GetBestMove;

impl Handler<GetBestMove> for Game {
    type Result = Option<LongAlgebricNotationMove>;

    fn handle(&mut self, _msg: GetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        self.best_move
    }
}

#[derive(Message)]
#[rtype(result = "Result<configuration::Configuration, ()>")]
pub struct GetConfiguration;

impl Handler<GetConfiguration> for Game {
    type Result = Result<configuration::Configuration, ()>;

    fn handle(&mut self, _msg: GetConfiguration, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.configuration().clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct PlayMoves(pub Vec<LongAlgebricNotationMove>);

impl Handler<PlayMoves> for Game {
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
    Stop,
}

pub type GameActor = Addr<Game>;

#[derive(Debug, Default)]
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
pub struct Game {
    configuration: configuration::Configuration,
    best_move: Option<LongAlgebricNotationMove>,
    history: History,
}
impl Game {
    pub fn new() -> Self {
        Game {
            configuration: configuration::Configuration::default(),
            best_move: None,
            history: History::default(),
        }
    }
    pub fn configuration(&self) -> &configuration::Configuration {
        &self.configuration
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
                        bit_position = bit_position.move_piece(&b_move);
                        self.configuration.update_position(bit_position.to());
                        self.history.add_moves(b_move);
                    }
                }
            }
        } else {
            result = Err("moves ignored since no position has been defined".to_string());
        }
        result
    }
}
impl Actor for Game {
    type Context = Context<Self>;
}

impl Handler<UciCommand> for Game {
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
            UciCommand::Stop => match self.configuration.opt_position() {
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
        (square::Square::Empty, _) => Err(format!("empty start square {}", m.start())),
        (square::Square::NonEmpty(piece), square::Square::Empty) => {
            let b_move = BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion());
            println!("check level2 for {:?}", m.cast());
            check_move_level2(b_move, bitboard_position)
        },
        (square::Square::NonEmpty(piece), square::Square::NonEmpty(capture)) if capture.color() != piece.color() => Ok(BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion())),
        (square::Square::NonEmpty(_), square::Square::NonEmpty(_)) => Err(format!("Invalid move from {} to {} since the destination square contains a piece of the same color as the piece played." , m.start(), m.end())),
    }
}

fn check_move_level2(
    b_move: BitBoardMove,
    bitboard_position: &bitboard::BitPosition,
) -> Result<BitBoardMove, String> {
    let color = bitboard_position.bit_position_status().player_turn();
    let bit_position_status = bitboard_position.bit_position_status();
    let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
    let check_status = bit_boards_white_and_black.check_status(&color, bit_position_status);
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
        Err(format!("The move {:?} is invalid.", b_move))
    }
}
