pub mod configuration;
pub mod parameters;

use crate::board::bitboard::BitBoardMove;
use crate::board::{bitboard, fen, square};
use crate::uci::notation::{self, LongAlgebricNotationMove};
use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "Option<LongAlgebricNotationMove>")]
pub struct GetBestMove;

impl Handler<GetBestMove> for Game {
    type Result = Option<LongAlgebricNotationMove>;

    fn handle(&mut self, msg: GetBestMove, ctx: &mut Self::Context) -> Self::Result {
        self.best_move
    }
}

#[derive(Message)]
#[rtype(result = "Result<configuration::Configuration, ()>")]
pub struct GetConfiguration;

impl Handler<GetConfiguration> for Game {
    type Result = Result<configuration::Configuration, ()>;

    fn handle(&mut self, msg: GetConfiguration, ctx: &mut Self::Context) -> Self::Result {
        Ok(self.configuration().clone())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), String>")]
pub enum UciCommand {
    InitPosition,
    UpdatePosition(fen::Position),
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

// Actor definition
pub struct Game {
    configuration: configuration::Configuration,
    best_move: Option<LongAlgebricNotationMove>,
}
impl Game {
    pub fn new() -> Self {
        Game {
            configuration: configuration::Configuration::default(),
            best_move: None,
        }
    }
    pub fn configuration(&self) -> &configuration::Configuration {
        &self.configuration
    }
    pub fn set_configuration(&mut self, configuration: configuration::Configuration) {
        self.configuration = configuration;
    }
}
impl Actor for Game {
    type Context = Context<Self>;
}

impl Handler<UciCommand> for Game {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UciCommand, ctx: &mut Self::Context) -> Self::Result {
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
            UciCommand::UpdatePosition(position) => {
                self.configuration.update_position(position);
            }
            UciCommand::SearchMoves(search_moves) => {
                let mut params = self.configuration().parameters().clone();
                params.set_search_moves(search_moves);
                self.configuration.update_parameters(params);
            }
            UciCommand::ValidMoves(valid_moves) => {
                if let Some(position) = self.configuration.opt_position() {
                    let mut bit_position = bitboard::BitPosition::from(position);
                    for m in valid_moves {
                        let color = bit_position.bit_position_status().player_turn();
                        match check_move(color, m, &bit_position.bit_boards_white_and_black()) {
                            Err(err) => {
                                result = Err(err);
                                break;
                            }
                            Ok(b_move) => {
                                bit_position = bit_position.move_piece(&b_move);
                                self.configuration.update_position(bit_position.to());
                            }
                        }
                    }
                } else {
                    result = Err("moves ignored since no position has been defined".to_string());
                }
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
    bitboard_white_and_black: &bitboard::BitBoardsWhiteAndBlack,
) -> Result<BitBoardMove, String> {
    let start_square = bitboard_white_and_black.peek(m.start());
    let end_square = bitboard_white_and_black.peek(m.end());
    match (start_square, end_square) {
        (square::Square::Empty, _) => Err(format!("empty start square {}", m.start())),
        (square::Square::NonEmpty(piece), square::Square::Empty) => Ok(BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion())),
        (square::Square::NonEmpty(piece), square::Square::NonEmpty(capture)) if capture.color() != piece.color() => Ok(BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion())),
        (square::Square::NonEmpty(_), square::Square::NonEmpty(_)) => Err(format!("Invalid move from {} to {} since the destination square contains a piece of the same color as the piece played." , m.start(), m.end())),
    }
}

pub fn moves_validation(
    moves: &Vec<String>,
) -> Result<Vec<notation::LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<notation::LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match notation::LongAlgebricNotationMove::build_from_str(&m) {
            Ok(valid_move) => valid_moves.push(valid_move),
            Err(err) => errors.push(err),
        }
    }
    if !errors.is_empty() {
        Err(errors.join(", "))
    } else {
        Ok(valid_moves)
    }
}
