pub mod configuration;
pub mod parameters;

use crate::board::bitboard::piece_move::{CheckStatus, GenMoves};
use crate::board::bitboard::BitBoardMove;
use crate::board::{bitboard, fen, square};
use crate::uci::notation::{self, LongAlgebricNotationMove};
use actix::prelude::*;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EndGame {
    #[default]
    None,
    Pat,
    Mat,
}

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
#[rtype(result = "Result<History, ()>")]
pub struct GetHistory;

impl Handler<GetHistory> for Game {
    type Result = Result<History, ()>;

    fn handle(&mut self, _msg: GetHistory, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.history().clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<EndGame, ()>")]
pub struct GetEndGame;

impl Handler<GetEndGame> for Game {
    type Result = Result<EndGame, ()>;

    fn handle(&mut self, _msg: GetEndGame, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.end_game().clone())
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
pub struct Game {
    configuration: configuration::Configuration,
    best_move: Option<LongAlgebricNotationMove>,
    history: History,
    // once a move is played, we update moves for the next player
    moves: Vec<BitBoardMove>,
    end_game: EndGame,
}
impl Game {
    pub fn new() -> Self {
        let mut game = Game::default();
        game.update_moves();
        game
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
            let color = bitboard_position.bit_position_status().player_turn();
            let bit_position_status = bitboard_position.bit_position_status();
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
            }
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
                        //println!("play: {:?}", b_move);
                        bit_position = bit_position.move_piece(&b_move);
                        self.configuration.update_position(bit_position.to());
                        self.history.add_moves(b_move);
                        //self.show();
                        self.update_moves();
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

    use super::configuration;

    async fn get_configuration(game_actor: &game::GameActor) -> configuration::Configuration {
        let result = game_actor.send(game::GetConfiguration).await.unwrap();
        result.unwrap()
    }

    #[actix::test]
    async fn test_game_capture_en_passant() {
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 e7e5 d5e6", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::start(game::Game::new());
        // unwrap() is the test
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let configuration = get_configuration(&game_actor).await;
        assert!(configuration.opt_position().is_some());
    }
    #[actix::test]
    async fn test_game_pawn_move_invalid() {
        let inputs = vec!["position startpos moves e2e4 e7e5 e4e5", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::start(game::Game::new());
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
        let game_actor = game::Game::start(game::Game::new());
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
        let game_actor = game::Game::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_pat_black_first() {
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K b - - 0 1", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_weird() {
        let inputs = vec!["position startpos moves d2d4 d7d5 b1c3 a7a6 c1f4 a6a5 d1d2 a5a4 e1c1 a4a3 h2h3 a3b2 c1b1 a8a2 h3h4 a2a1 b1b2", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor = game::Game::start(game::Game::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        //let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        //assert_eq!(end_game, game::EndGame::Pat)
    }
}
