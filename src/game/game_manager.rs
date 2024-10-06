use actix::{
    dev::ContextFutureSpawner, Actor, ActorFutureExt, Addr, Context, Handler, Message,
    ResponseFuture, WrapFuture,
};

use crate::{
    board::{
        bitboard::{self, zobrist},
        fen,
        square::{self, Switch},
    },
    uci::notation::{self, LongAlgebricNotationMove},
};

use super::{chessclock, engine, game_state::GameState, parameters, player};

pub type GameManagerActor<T> = Addr<GameManager<T>>;

#[derive(Debug, Default, Clone)]
pub struct History {
    fen: String,
    moves: Vec<bitboard::BitBoardMove>,
}
impl History {
    pub fn init(&mut self) {
        self.set_fen(fen::FEN_START_POSITION);
    }
    pub fn set_fen(&mut self, fen: &str) {
        self.fen = fen.to_string();
        self.moves = vec![];
    }
    pub fn add_moves(&mut self, m: bitboard::BitBoardMove) {
        self.moves.push(m);
    }
}

#[derive(Default)]
pub struct GameManager<T: engine::EngineActor> {
    game_state_opt: Option<GameState>,
    best_move_opt: Option<LongAlgebricNotationMove>, // TODO: remove ?
    history: History,
    parameters: parameters::Parameters,
    players: player::Players<T>,
    white_clock_actor_opt: Option<chessclock::ClockActor<T>>,
    black_clock_actor_opt: Option<chessclock::ClockActor<T>>,
    zobrist_table: zobrist::Zobrist,
}

impl<T: engine::EngineActor> GameManager<T> {
    pub fn new() -> Self {
        let mut game_manager = GameManager::default();
        game_manager.zobrist_table = zobrist::Zobrist::new();
        game_manager
    }
    pub fn game_state(&self) -> Option<&GameState> {
        self.game_state_opt.as_ref()
    }
    pub fn history(&self) -> &History {
        &self.history
    }
}

impl<T: engine::EngineActor> Actor for GameManager<T> {
    type Context = Context<Self>;
}

impl<T: engine::EngineActor> Handler<UciCommand> for GameManager<T> {
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
                self.parameters.set_btime(time);
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
                self.parameters.set_btime_inc(time_inc);
            }
            UciCommand::InitPosition => {
                let position = fen::Position::build_initial_position();
                self.game_state_opt = Some(super::game_state::GameState::new(
                    position,
                    &self.zobrist_table,
                ));
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
                self.parameters.set_wtime(time);
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
                self.parameters.set_wtime_inc(time_inc);
            }
            UciCommand::DepthFinite(depth) => {
                self.parameters.set_depth(depth);
            }
            UciCommand::SearchInfinite => {
                self.parameters.set_depth_infinite();
            }
            UciCommand::MaxTimePerMoveInMs(time) => {
                self.parameters.set_time_per_move_in_ms(time);
            }
            UciCommand::UpdatePosition(fen, position) => {
                self.game_state_opt = Some(GameState::new(position, &self.zobrist_table));
                self.history.set_fen(&fen);
                self.best_move_opt = None;
            }
            UciCommand::SearchMoves(search_moves) => {
                self.parameters.set_search_moves(search_moves);
            }
            UciCommand::ValidMoves(valid_moves) => {
                result = self.play_moves(valid_moves);
            }
            UciCommand::StartEngine => {
                if let Some(ref game_state) = &self.game_state_opt {
                    let bit_position = game_state.bit_position();
                    let engine_actor_or_error = self
                        .players
                        .get_engine(bit_position.bit_position_status().player_turn());
                    match engine_actor_or_error {
                        Ok(engine_actor) => {
                            let msg = engine::EngineGo::new(bit_position.clone());
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
            UciCommand::StopEngine => match &self.game_state_opt {
                None => {
                    self.best_move_opt = None;
                    result =
                        Err("No bestmove since no valid position has been entered.".to_string());
                }
                Some(game_state) => {
                    match self.players.get_engine(
                        game_state
                            .bit_position()
                            .bit_position_status()
                            .player_turn(),
                    ) {
                        Ok(engine_actor) => {
                            let engine_msg = engine::EngineGetBestMove::default();

                            engine_actor
                                .send(engine_msg)
                                .into_actor(self)
                                .map(move |result: Result<Option<bitboard::BitBoardMove>, _>, act, _ctx| {
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
                } // Stop engine search
            },
        }
        result
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
impl<T: engine::EngineActor> Handler<SetClocks<T>> for GameManager<T> {
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

impl<T: engine::EngineActor> GameManager<T> {
    fn play_moves(&mut self, valid_moves: Vec<LongAlgebricNotationMove>) -> Result<(), String> {
        let result: Option<Result<Vec<bitboard::BitBoardMove>, String>> = self
            .game_state_opt
            .as_mut()
            .and_then(|game_state: &mut GameState| {
                Some(game_state.play_moves(valid_moves, &self.zobrist_table))
            });
        match result {
            Some(Ok(b_moves)) => {
                let mut n_moves_white = 0u64;
                let mut n_moves_black = 0u64;
                for b_move in b_moves {
                    self.history.add_moves(b_move);
                    if b_move.color() == square::Color::White {
                        n_moves_white += 1;
                    } else {
                        n_moves_black += 1;
                    }
                }
                if let Some(white_clock_actor) = &self.white_clock_actor_opt {
                    async_clock_inc(
                        "white".to_string(),
                        n_moves_white,
                        white_clock_actor.clone(),
                    );
                }
                if let Some(black_clock_actor) = &self.black_clock_actor_opt {
                    async_clock_inc(
                        "black".to_string(),
                        n_moves_black,
                        black_clock_actor.clone(),
                    );
                }
                Ok(())
            }
            Some(Err(err)) => Err(err), // illegal move
            None => Err("moves ignored since no position has been defined".to_string()),
        }
    }
    pub fn set_players(&mut self, players: player::Players<T>) {
        self.players = players;
    }
}

fn async_clock_inc<T: engine::EngineActor>(
    debug: String,
    n_moves: u64,
    clock_actor: Addr<chessclock::Clock<T>>,
) {
    use tokio::task;

    // Offload the sending to a background task
    task::spawn(async move {
        let result = clock_actor
            .send(chessclock::IncRemainingTime(n_moves))
            .await;
        match result {
            Ok(response) => println!(
                "Time for {} incremented successfully: {:?}",
                debug, response
            ),
            Err(e) => println!("Error incrementing time: {:?}", e),
        }
    });
}

impl<T: engine::EngineActor> Handler<chessclock::TimeOut> for GameManager<T> {
    type Result = ();

    fn handle(&mut self, _msg: chessclock::TimeOut, _ctx: &mut Context<Self>) {
        println!("Time is up !");
        if let Some(ref mut game_state) = &mut self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
            if game_state
                .check_insufficient_material_for_color(color.switch(), bit_boards_white_and_black)
            {
                game_state.set_end_game(super::game_state::EndGame::TimeOutDraw);
                println!("set end game TimeOutDraw");
            } else {
                game_state.set_end_game(super::game_state::EndGame::TimeOutLost(color));
                println!("set end game: TimeOutLost");
            }
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

impl<T: engine::EngineActor> Handler<GetClockRemainingTime> for GameManager<T> {
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

impl<T: engine::EngineActor> Handler<SetClockRemainingTime> for GameManager<T> {
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
impl<T: engine::EngineActor> Handler<StartOrSwitchClocks> for GameManager<T> {
    type Result = ();

    fn handle(&mut self, _msg: StartOrSwitchClocks, _ctx: &mut Context<Self>) {
        if let Some(game_state) = &self.game_state_opt {
            let bitboard_position = game_state.bit_position();
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

impl<T: engine::EngineActor> Handler<GetBestMove> for GameManager<T> {
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

impl<T: engine::EngineActor> Handler<GetCurrentEngine<T>> for GameManager<T> {
    type Result = Option<Addr<T>>;

    fn handle(&mut self, _msg: GetCurrentEngine<T>, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(game_state) = &self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            self.players.get_player_into(color).get_engine().cloned()
        } else {
            None
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<super::game_state::EndGame, ()>")]
pub struct GetEndGame;

impl<T: engine::EngineActor> Handler<GetEndGame> for GameManager<T> {
    type Result = Result<super::game_state::EndGame, ()>;

    fn handle(&mut self, _msg: GetEndGame, _ctx: &mut Self::Context) -> Self::Result {
        println!("end game status");
        let end_game = match &self.game_state_opt {
            None => super::game_state::EndGame::None,
            Some(game_state) => game_state.end_game(),
        };
        Ok(end_game)
    }
}

#[derive(Message)]
#[rtype(result = "Option<GameState>")]
pub struct GetGameState;

impl<T: engine::EngineActor> Handler<GetGameState> for GameManager<T> {
    type Result = Option<GameState>;

    fn handle(&mut self, _msg: GetGameState, _ctx: &mut Self::Context) -> Self::Result {
        self.game_state().cloned()
    }
}

#[derive(Message)]
#[rtype(result = "Result<super::parameters::Parameters, ()>")]
pub struct GetParameters;

impl<T: engine::EngineActor> Handler<GetParameters> for GameManager<T> {
    type Result = Result<super::parameters::Parameters, ()>;

    fn handle(&mut self, _msg: GetParameters, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.parameters.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), String>")]
pub struct PlayMoves(pub Vec<LongAlgebricNotationMove>);

impl<T: engine::EngineActor> Handler<PlayMoves> for GameManager<T> {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PlayMoves, _ctx: &mut Self::Context) -> Self::Result {
        self.play_moves(msg.0)
    }
}

#[derive(Message)]
#[rtype(result = "Result<History, ()>")]
pub struct GetHistory;

impl<T: engine::EngineActor> Handler<GetHistory> for GameManager<T> {
    type Result = Result<History, ()>;

    fn handle(&mut self, _msg: GetHistory, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.history().clone())
    }
}

#[cfg(test)]
pub async fn build_game_actor(inputs: Vec<&str>) -> GameManagerActor<engine::EngineDummy> {
    let uci_reader = crate::uci::UciReadVecStringWrapper::new(inputs.as_slice());
    let mut game = GameManager::<engine::EngineDummy>::new();
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
    let game_manager_actor = GameManager::<engine::EngineDummy>::start(game);
    // set the position from uci command
    crate::uci::uci_loop(uci_reader, &game_manager_actor)
        .await
        .unwrap();
    // define clocks
    let white_clock_actor =
        chessclock::Clock::new("white", 3, 0, game_manager_actor.clone()).start();
    let black_clock_actor =
        chessclock::Clock::new("black", 3, 0, game_manager_actor.clone()).start();
    game_manager_actor.do_send(SetClocks {
        white_clock_actor_opt: Some(white_clock_actor),
        black_clock_actor_opt: Some(black_clock_actor),
    });
    // send clock to game
    let set_clock_msg = SetClockRemainingTime {
        color: square::Color::White,
        remaining_time: 2,
    };
    game_manager_actor.do_send(set_clock_msg);

    game_manager_actor
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use actix::Actor;

    use crate::game::{chessclock, game_manager, game_state, player};
    use crate::{board::square, uci};

    use super::{engine, GameManager};

    #[actix::test]
    async fn test_game_capture_en_passant() {
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 e7e5 d5e6", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        // unwrap() is the test
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let game_state_opt = game_manager_actor
            .send(game_manager::GetGameState)
            .await
            .unwrap();
        assert!(game_state_opt.is_some());
    }
    #[actix::test]
    async fn test_game_pawn_move_invalid() {
        let inputs = vec!["position startpos moves e2e4 e7e5 e4e5", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        let r = uci::uci_loop(uci_reader, &game_manager_actor).await;
        assert!(r.is_err());
    }

    #[actix::test]
    async fn test_game_mat() {
        let inputs = vec![
            "position startpos moves e2e4 e7e5 f1c4 a7a6 d1f3 a6a5 f3f7",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Mat(square::Color::Black))
    }
    #[actix::test]
    async fn test_game_pat_white_first() {
        let inputs = vec![
            "position fen k7/7R/1R6/8/8/8/8/7K w - - 0 1 moves h1g1",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_pat_black_first() {
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K b - - 0 1", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = game_manager::GameManager::<engine::EngineDummy>::start(
            game_manager::GameManager::new(),
        );
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_weird() {
        let inputs = vec!["position startpos moves d2d4 d7d5 b1c3 a7a6 c1f4 a6a5 d1d2 a5a4 e1c1 a4a3 h2h3 a3b2 c1b1 a8a2 h3h4 a2a1 b1b2", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        let result = uci::uci_loop(uci_reader, &game_manager_actor).await;
        assert!(result.is_ok())
    }
    #[actix::test]
    async fn test_game_blocked_pawn_ckeck() {
        let inputs = vec!["position startpos moves e2e4 e7e5 a2a3 d8h4 f2f3", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor =
            game_manager::GameManager::<engine::EngineDummy>::start(GameManager::new());
        let result = uci::uci_loop(uci_reader, &game_manager_actor).await;
        assert!(result.is_err())
    }
    #[actix::test]
    async fn test_game_block_ckeck() {
        let inputs = vec![
            "position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1f3",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = game_manager::GameManager::<engine::EngineDummy>::start(
            game_manager::GameManager::new(),
        );
        let result = uci::uci_loop(uci_reader, &game_manager_actor).await;
        assert!(result.is_err())
    }
    #[actix::test]
    async fn test_game_block_ckeck2() {
        let inputs = vec![
            "position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1e2",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor =
            game_manager::GameManager::<engine::EngineDummy>::start(GameManager::new());
        let result = uci::uci_loop(uci_reader, &game_manager_actor).await;
        assert!(result.is_ok())
    }
    #[actix::test]
    async fn test_game_escape() {
        let inputs = vec![
            "position startpos moves e2e4 c7c5 f1c4 d7d6 d1h5 a7a6 h5f7 e8d7",
            "quit",
        ];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor =
            game_manager::GameManager::<engine::EngineDummy>::start(GameManager::new());
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
        let game_actor =
            game_manager::GameManager::<engine::EngineDummy>::start(GameManager::new());
        let result = uci::uci_loop(uci_reader, &game_actor).await;
        assert!(result.is_err())
    }
    #[actix::test]
    async fn test_game_rule_insufficient_material() {
        let inputs = vec!["position fen k7/8/8/8/8/8/8/7K b - - 0 1", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_actor =
            game_manager::GameManager::<engine::EngineDummy>::start(GameManager::new());
        uci::uci_loop(uci_reader, &game_actor).await.unwrap();
        let end_game = game_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::InsufficientMaterial)
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
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::NoPawnAndCapturex50)
    }

    #[actix::test]
    async fn test_game_3x_position() {
        let moves = "h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!("position fen k7/8/r7/8/8/7R/8/7K w - - 0 1 moves {}", moves);
        let inputs = vec![&fen, "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Repetition3x)
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
        let game_manager_actor = GameManager::<engine::EngineDummy>::start(GameManager::new());
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Repetition3x)
    }

    #[actix::test]
    async fn test_game_timeout_gameover() {
        let inputs = vec!["position startpos", "quit"];
        let game_manager_actor = game_manager::build_game_actor(inputs).await;
        game_manager_actor.do_send(game_manager::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(3)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            end_game,
            game_state::EndGame::TimeOutLost(square::Color::White)
        )
    }
    #[actix::test]
    async fn test_game_inc_timer() {
        let inputs = vec!["position startpos moves e2e4 e7e5 g1f3 g8f6 f1c4", "quit"];
        let uci_reader = uci::UciReadVecStringWrapper::new(inputs.as_slice());
        let mut game_manager = super::GameManager::<engine::EngineDummy>::new();
        let engine_player1 = engine::EngineDummy::default().start();
        let engine_player2 = engine::EngineDummy::default().start();
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2,
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor =
            game_manager::GameManager::<engine::EngineDummy>::start(game_manager);
        // set the position from uci command
        let white_clock_actor =
            chessclock::Clock::new("white", 3, 1, game_manager_actor.clone()).start();
        let black_clock_actor =
            chessclock::Clock::new("black", 4, 2, game_manager_actor.clone()).start();
        game_manager_actor.do_send(game_manager::SetClocks {
            white_clock_actor_opt: Some(white_clock_actor),
            black_clock_actor_opt: Some(black_clock_actor),
        });
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        uci::uci_loop(uci_reader, &game_manager_actor)
            .await
            .unwrap();
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let clock_white = game_manager_actor
            .send(game_manager::GetClockRemainingTime::new(
                square::Color::White,
            ))
            .await
            .unwrap()
            .unwrap();
        let clock_black = game_manager_actor
            .send(game_manager::GetClockRemainingTime::new(
                square::Color::Black,
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(clock_white, 6);
        assert_eq!(clock_black, 8);
    }
    #[actix::test]
    async fn test_game_timeout_no_material_gameover() {
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K w - - 0 1", "quit"];
        let game_manager_actor = game_manager::build_game_actor(inputs).await;
        game_manager_actor.do_send(game_manager::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(3)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            end_game,
            game_state::EndGame::TimeOutLost(square::Color::White)
        )
    }
    #[actix::test]
    async fn test_game_opponent_timeout_no_material_draw() {
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K b - - 0 1", "quit"];
        let game_manager_actor = game_manager::build_game_actor(inputs).await;
        game_manager_actor.do_send(game_manager::StartOrSwitchClocks);
        actix::clock::sleep(Duration::from_secs(4)).await;
        // Introduce a delay to ensure the TimeOut message is processed
        actix::clock::sleep(std::time::Duration::from_millis(100)).await;
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::TimeOutDraw)
    }
}
