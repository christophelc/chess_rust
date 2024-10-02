pub mod chessclock;
pub mod configuration;
pub mod engine;
pub mod parameters;
pub mod player;

use crate::board::bitboard::piece_move::{CheckStatus, GenMoves};
use crate::board::bitboard::{zobrist, BitBoardMove, BitBoardsWhiteAndBlack};
use crate::board::square::Switch;
use crate::board::{bitboard, fen, square};
use crate::uci::notation::{self, LongAlgebricNotationMove};
use actix::prelude::*;

#[derive(Debug, Default, Clone, PartialEq)]
pub enum EndGame {
    #[default]
    None,
    Pat,
    Mat(square::Color),
    NoPawnAndCapturex50,  // 50 moves rule
    InsufficientMaterial, // King (+ bishop or knight) vs King (+ bishop or knight)
    Repetition3x,         // 3x the same position
    TimeOutLost(square::Color),
    TimeOutDraw,   // Timeout but only a King, King + Bishop or Knight
    NullAgreement, // Two players agree to end the game
}

impl<T: engine::EngineActor> Handler<chessclock::TimeOut> for Game<T> {
    type Result = ();

    fn handle(&mut self, _msg: chessclock::TimeOut, _ctx: &mut Context<Self>) {
        println!("Time is up !");
        if let Some(position) = self.configuration.opt_position() {
            let bitboard_position = bitboard::BitPosition::from(position);
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
            if self
                .check_insufficient_material_for_color(color.switch(), bit_boards_white_and_black)
            {
                self.end_game = EndGame::TimeOutDraw
            } else {
                self.end_game = EndGame::TimeOutLost(color)
            }
            println!("set end game: {:?}", self.end_game);
        } else {
            panic!("A clock has been started but no position has been set.")
        }
    }
}

#[derive(Message)]
#[rtype(result = "Option<u64>")]
pub struct GetClockRemainingTime(square::Color);

#[cfg(test)]
impl GetClockRemainingTime {
    pub fn new(color: square::Color) -> Self {
        GetClockRemainingTime(color)
    }
}

impl<T: engine::EngineActor> Handler<GetClockRemainingTime> for Game<T> {
    type Result = ResponseFuture<Option<u64>>;

    fn handle(&mut self, msg: GetClockRemainingTime, _ctx: &mut Self::Context) -> Self::Result {
        let white_clock_actor_opt = self.white_clock_actor_opt.clone();
        let black_clock_actor_opt = self.black_clock_actor_opt.clone();
        Box::pin(async move {
            match (msg.0, white_clock_actor_opt, black_clock_actor_opt) {
                (square::Color::White, Some(white_clock_actor), _) => {
                    let result = white_clock_actor
                        .send(chessclock::GetRemainingTime)
                        .await
                        .ok()?;
                    Some(result)
                }
                (square::Color::Black, _, Some(black_clock_actor)) => {
                    let result = black_clock_actor
                        .send(chessclock::GetRemainingTime)
                        .await
                        .ok()?;
                    Some(result)
                }
                _ => None,
            }
        })
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetClockRemainingTime {
    color: square::Color,
    remaining_time: u64,
}

impl<T: engine::EngineActor> Handler<SetClockRemainingTime> for Game<T> {
    type Result = ();

    fn handle(&mut self, msg: SetClockRemainingTime, _ctx: &mut Self::Context) -> Self::Result {
        match msg.color {
            square::Color::White => self
                .white_clock_actor_opt
                .as_mut()
                .unwrap()
                .do_send(chessclock::SetRemainingTime::new(msg.remaining_time)),
            square::Color::Black => self
                .black_clock_actor_opt
                .as_mut()
                .unwrap()
                .do_send(chessclock::SetRemainingTime::new(msg.remaining_time)),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct StartOrSwitchClocks;

// Implementing a handler for starting the clocks
impl<T: engine::EngineActor> Handler<StartOrSwitchClocks> for Game<T> {
    type Result = ();

    fn handle(&mut self, _msg: StartOrSwitchClocks, _ctx: &mut Context<Self>) {
        if let Some(position) = self.configuration.opt_position() {
            let bitboard_position = bitboard::BitPosition::from(position);
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            if self.white_clock_actor_opt.is_none() || self.black_clock_actor_opt.is_none() {
                panic!("Cannot start clocks. No clock has been defined.")
            }
            match color {
                square::Color::White => {
                    println!("Pause black, resume white");
                    self.black_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::PauseClock);
                    self.white_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::ResumeClock);
                }
                square::Color::Black => {
                    println!("Pause white, resume black");
                    self.black_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::ResumeClock);
                    self.white_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::PauseClock);
                }
            }
        } else {
            panic!("Try to start clocks whereas no position has been detected.")
        }
    }
}

#[derive(Message)]
#[rtype(result = "Option<LongAlgebricNotationMove>")]
pub struct GetBestMove;

impl<T: engine::EngineActor> Handler<GetBestMove> for Game<T> {
    type Result = Option<LongAlgebricNotationMove>;

    fn handle(&mut self, _msg: GetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        self.best_move_opt
    }
}

#[derive(Default)]
pub struct GetCurrentEngine<T> {
    _phantom: std::marker::PhantomData<T>,
}
impl<T: engine::EngineActor> Message for GetCurrentEngine<T> {
    type Result = Option<Addr<T>>;
}

impl<T: engine::EngineActor> Handler<GetCurrentEngine<T>> for Game<T> {
    type Result = Option<Addr<T>>;

    fn handle(&mut self, _msg: GetCurrentEngine<T>, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(position) = self.configuration.opt_position() {
            let bitboard_position = bitboard::BitPosition::from(position);
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            self.players.get_player_into(color).get_engine().cloned()
        } else {
            None
        }
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
        println!("end game status");
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
    MaxTimePerMoveInMs(u32),
    SearchInfinite,
    Wtime(u64),
    Btime(u64),
    WtimeInc(u64),
    BtimeInc(u64),
    SearchMoves(Vec<notation::LongAlgebricNotationMove>),
    ValidMoves(Vec<notation::LongAlgebricNotationMove>),
    StartEngine,
    StopEngine,
}

// Message to set the clocks in the Game actor
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetClocks<T: engine::EngineActor> {
    white_clock_actor_opt: Option<chessclock::ClockActor<T>>,
    black_clock_actor_opt: Option<chessclock::ClockActor<T>>,
}
#[cfg(test)]
impl<T: engine::EngineActor> SetClocks<T> {
    pub fn new(
        white_clock_actor_opt: Option<chessclock::ClockActor<T>>,
        black_clock_actor_opt: Option<chessclock::ClockActor<T>>,
    ) -> Self {
        SetClocks {
            white_clock_actor_opt,
            black_clock_actor_opt,
        }
    }
}
impl<T: engine::EngineActor> Handler<SetClocks<T>> for Game<T> {
    type Result = ();

    fn handle(&mut self, msg: SetClocks<T>, _ctx: &mut Context<Self>) {
        // If white clock exists, terminate it before setting a new one
        if let Some(clock_actor) = &self.white_clock_actor_opt {
            clock_actor.do_send(chessclock::TerminateClock);
        }

        // If black clock exists, terminate it before setting a new one
        if let Some(clock_actor) = &self.black_clock_actor_opt {
            clock_actor.do_send(chessclock::TerminateClock);
        }

        // Set the new clock actors from the message
        self.white_clock_actor_opt = msg.white_clock_actor_opt;
        self.black_clock_actor_opt = msg.black_clock_actor_opt;
    }
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
    best_move_opt: Option<LongAlgebricNotationMove>,
    history: History,
    // once a move is played, we update moves for the next player
    moves: Vec<BitBoardMove>,
    hash_positions: zobrist::ZobristHistory,
    end_game: EndGame,
    zobrist_table: zobrist::Zobrist,
    players: player::Players<T>,
    white_clock_actor_opt: Option<chessclock::ClockActor<T>>,
    black_clock_actor_opt: Option<chessclock::ClockActor<T>>,
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
                    _ => self.end_game = EndGame::Mat(color),
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
    fn check_insufficient_material_for_color(
        &self,
        color: square::Color,
        bit_boards_white_and_black: &BitBoardsWhiteAndBlack,
    ) -> bool {
        let bitboard = bit_boards_white_and_black.bit_board(&color);
        let relevant_pieces = *bitboard.rooks().bitboard()
            | *bitboard.queens().bitboard()
            | *bitboard.pawns().bitboard();
        // one bishop or knight only
        if relevant_pieces.empty() {
            let other = *bitboard.bishops().bitboard() | *bitboard.knights().bitboard();
            other.one_bit_set_max()
        } else {
            false
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
    fn play_moves(&mut self, valid_moves: Vec<LongAlgebricNotationMove>) -> Result<(), String> {
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

    pub fn set_players(&mut self, players: player::Players<T>) {
        self.players = players;
    }
}
impl<T: engine::EngineActor> Actor for Game<T> {
    type Context = Context<Self>;
}

impl<T: engine::EngineActor> Handler<UciCommand> for Game<T> {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UciCommand, ctx: &mut Self::Context) -> Self::Result {
        let mut result = Ok(());
        match msg {
            UciCommand::Btime(time) => {
                match &self.black_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(clock_actor) => {
                        clock_actor.do_send(chessclock::SetRemainingTime::new(time));
                    }
                }
                // for the moment, we memorize the inital parameters
                let mut params = self.configuration().parameters().clone();
                params.set_btime(time);
                self.configuration.update_parameters(params);
            }
            UciCommand::BtimeInc(time_inc) => {
                match &self.black_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(clock_actor) => {
                        clock_actor.do_send(chessclock::SetIncTime::new(time_inc));
                    }
                }
                // for the moment, we memorize the inital parameters
                let mut params = self.configuration().parameters().clone();
                params.set_btime_inc(time_inc);
                self.configuration.update_parameters(params);
            }
            UciCommand::InitPosition => {
                let position = fen::Position::build_initial_position();
                self.configuration.update_position(position);
                self.init_hash_table();
                self.history.init();
                self.best_move_opt = None;
            }
            UciCommand::Wtime(time) => {
                match &self.white_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(white_clock_actor) => {
                        white_clock_actor.do_send(chessclock::SetRemainingTime::new(time));
                    }
                }
                // for the moment, we memorize the inital parameters
                let mut params = self.configuration().parameters().clone();
                params.set_wtime(time);
                self.configuration.update_parameters(params);
            }
            UciCommand::WtimeInc(time_inc) => {
                match &self.white_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(white_clock_actor) => {
                        white_clock_actor.do_send(chessclock::SetIncTime::new(time_inc));
                    }
                }
                // for the moment, we memorize the inital parameters
                let mut params = self.configuration().parameters().clone();
                params.set_wtime_inc(time_inc);
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
            UciCommand::MaxTimePerMoveInMs(time) => {
                let mut params = self.configuration().parameters().clone();
                params.set_time_per_move_in_ms(time);
                self.configuration.update_parameters(params);
            }
            UciCommand::UpdatePosition(fen, position) => {
                self.configuration.update_position(position);
                self.init_hash_table();
                self.update_moves();
                self.history.set_fen(&fen);
                self.best_move_opt = None;
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
                        Ok(engine_actor) => {
                            let msg = engine::EngineGo::new(bitboard::BitPosition::from(position));
                            engine_actor
                                .send(msg)
                                .into_actor(self)
                                .map(|result, _act, _ctx| match result {
                                    Ok(_) => {}
                                    Err(e) => println!("Failed to send message: {:?}", e),
                                })
                                .wait(ctx); // Wait for the future to complete within the actor context
                        }
                        Err(err) => result = Err(err),
                    }
                }
            }
            UciCommand::StopEngine => match self.configuration.opt_position() {
                None => {
                    self.best_move_opt = None;
                    result =
                        Err("No bestmove since no valid position has been entered.".to_string());
                }
                Some(_) => {
                    if let Some(position) = self.configuration().opt_position() {
                        match self.players.get_engine(position.status().player_turn()) {
                            Ok(engine_actor) => {
                                let engine_msg = engine::EngineGetBestMove::default();

                                engine_actor
                                    .send(engine_msg)
                                    .into_actor(self)
                                    .map(move |result: Result<Option<BitBoardMove>, _>, act, _ctx| {
                                        match result {
                                            Ok(Some(best_move)) => {
                                                println!("Best move updated successfully");
                                                act.best_move_opt = Some(notation::LongAlgebricNotationMove::build_from_b_move(best_move));
                                            }
                                            Ok(None) => {
                                                println!("No move found.");
                                                act.best_move_opt = None;
                                            }
                                            Err(e) => {
                                                println!("Error sending message to engine: {:?}", e);
                                                act.best_move_opt = None;
                                            }
                                        }
                                    })
                                    .wait(ctx); // Wait for the future to complete within the actor context
                            }
                            Err(err) => {
                                println!("Failed to retrieve engine actor: {:?}", err);
                                self.best_move_opt = None;
                            }
                        }
                    } else {
                        println!("No position found in configuration.");
                        self.best_move_opt = None;
                    }
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
    use std::time::Duration;

    use actix::Actor;

    use crate::{
        board::square,
        game::{self, chessclock, player},
        uci,
    };

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
        assert_eq!(end_game, game::EndGame::Mat(square::Color::Black))
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

    pub async fn build_game_actor(inputs: Vec<&str>) -> game::GameActor<engine::EngineDummy> {
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let mut game = game::Game::<engine::EngineDummy>::new();
        let engine_player1 = engine::EngineDummy::default().start();
        let engine_player2 = engine::EngineDummy::default().start();
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2,
        };
        let players = player::Players::new(player1, player2);
        game.set_players(players);
        let game_actor = game::Game::<engine::EngineDummy>::start(game);
        // set the position from uci command
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        // define clocks
        let white_clock_actor = chessclock::Clock::new(3, 0, game_actor.clone()).start();
        let black_clock_actor = chessclock::Clock::new(3, 0, game_actor.clone()).start();
        game_actor.do_send(game::SetClocks {
            white_clock_actor_opt: Some(white_clock_actor),
            black_clock_actor_opt: Some(black_clock_actor),
        });
        // send clock to game
        let set_clock_msg = game::SetClockRemainingTime {
            color: square::Color::White,
            remaining_time: 2,
        };
        game_actor.do_send(set_clock_msg);

        game_actor
    }
    #[actix::test]
    async fn test_game_timeout_gameover() {
        let inputs = vec!["position startpos", "quit"];
        let game_actor = build_game_actor(inputs).await;
        game_actor.do_send(game::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(3)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::TimeOutLost(square::Color::White))
    }
    #[actix::test]
    async fn test_game_timeout_no_material_gameover() {
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K w - - 0 1", "quit"];
        let game_actor = build_game_actor(inputs).await;
        game_actor.do_send(game::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(3)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::TimeOutLost(square::Color::White))
    }
    #[actix::test]
    async fn test_game_opponent_timeout_no_material_draw() {
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K b - - 0 1", "quit"];
        let game_actor = build_game_actor(inputs).await;
        game_actor.do_send(game::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(4)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_actor.send(game::GetEndGame).await.unwrap().unwrap();
        assert_eq!(end_game, game::EndGame::TimeOutDraw)
    }
}
