use actix::{
    dev::ContextFutureSpawner, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, Context,
    Handler, Message, ResponseFuture, WrapFuture,
};

use crate::entity::game::component::{
    bitboard::{self, zobrist},
    game_state,
    square::{self, Switch},
};
use crate::ui::notation::fen;
use crate::ui::notation::long_notation::{self, LongAlgebricNotationMove};
use crate::ui::uci;

use crate::entity::clock::actor::chessclock;
use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::engine::component::engine_logic as logic;
use crate::entity::game::component::{game_state::GameState, parameters, player};
use crate::monitoring::debug;

pub type GameManagerActor = Addr<GameManager>;

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

#[derive(Debug, Clone)]
pub struct TimestampedBestMove {
    best_move: long_notation::LongAlgebricNotationMove,
    timestamp: chrono::DateTime<chrono::Utc>, // date of best_move initialization
    engine_id: logic::EngineId,               // which engine has found the best move
}
impl TimestampedBestMove {
    fn build(
        best_move: long_notation::LongAlgebricNotationMove,
        timestamp: chrono::DateTime<chrono::Utc>,
        engine_id: logic::EngineId,
    ) -> Self {
        Self {
            best_move,
            timestamp,
            engine_id,
        }
    }
    pub fn best_move(&self) -> long_notation::LongAlgebricNotationMove {
        self.best_move
    }
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }
    pub fn origin(&self) -> logic::EngineId {
        self.engine_id.clone()
    }
    fn is_more_recent_best_move_than(&self, timestamped_best_move: &TimestampedBestMove) -> bool {
        self.timestamp > timestamped_best_move.timestamp
    }
}

#[derive(Default)]
pub struct GameManager {
    game_state_opt: Option<GameState>,
    debug_actor_opt: Option<debug::DebugActor>,
    ts_best_move_opt: Option<TimestampedBestMove>,
    history: History,
    parameters: parameters::Parameters,
    players: player::Players,
    white_clock_actor_opt: Option<chessclock::ClockActor>,
    black_clock_actor_opt: Option<chessclock::ClockActor>,
    zobrist_table: zobrist::Zobrist,
}

impl GameManager {
    pub fn new(debug_actor_opt: Option<debug::DebugActor>) -> Self {
        let mut game_manager = GameManager::default();
        game_manager.debug_actor_opt = debug_actor_opt;
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

impl Actor for GameManager {
    type Context = Context<Self>;
}

impl Handler<UciCommand> for GameManager {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UciCommand, ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive command: {:?}",
                msg
            )));
        }
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
                self.game_state_opt =
                    Some(game_state::GameState::new(position, &self.zobrist_table));
                self.history.init();
                self.ts_best_move_opt = None;
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
                self.ts_best_move_opt = None;
                if let Some(debug_actor) = &self.debug_actor_opt {
                    let msg = format!(
                        "New position is:\n{}",
                        self.game_state_opt
                            .as_ref()
                            .unwrap()
                            .bit_position()
                            .to()
                            .chessboard()
                    );
                    debug_actor.do_send(debug::AddMessage(msg));
                }
            }
            UciCommand::SearchMoves(search_moves) => {
                self.parameters.set_search_moves(search_moves);
            }
            UciCommand::ValidMoves { moves } => {
                result = self.play_moves(moves);
            }
            UciCommand::EngineStartThinking => {
                if let Some(ref game_state) = &self.game_state_opt {
                    let bit_position = game_state.bit_position();
                    let color = bit_position.bit_position_status().player_turn();
                    let engine_actor_or_error = self.players.get_engine(color);
                    match engine_actor_or_error {
                        Ok(engine_actor) => {
                            let msg = dispatcher::EngineStartThinking::new(
                                bit_position.clone(),
                                ctx.address().clone(),
                            );
                            if let Some(debug_actor) = &self.debug_actor_opt {
                                debug_actor.do_send(debug::AddMessage(format!(
                                    "game_manager_actor forward message to engine_actor for color {:?}: {:?}", color,
                                    msg
                                )));
                            }
                            engine_actor.do_send(msg);
                        }
                        Err(err) => result = Err(err),
                    }
                }
            }
            UciCommand::CleanResources => {
                if let Some(game_state) = &self.game_state_opt {
                    // clean resources for each engine actor
                    let color = game_state
                        .bit_position()
                        .bit_position_status()
                        .player_turn();
                    let engine_current_actor = self.players.get_engine(color).ok();
                    let engine_opponent_actor = self.players.get_engine(color.switch()).ok();
                    if let Some(debug_actor) = &self.debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(
                            "game_manager_actor forward message to engines_actor: EngineCleanResources".to_string()));
                    }
                    for engine_actor in engine_current_actor
                        .iter()
                        .chain(engine_opponent_actor.iter())
                    {
                        engine_actor
                            .send(dispatcher::EngineCleanResources)
                            .into_actor(self)
                            .map(|_result, _act, _ctx| ())
                            .wait(ctx);
                    }
                }
            }
            UciCommand::EngineStopThinking => match &self.game_state_opt {
                None => {
                    self.ts_best_move_opt = None;
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
                            // stop thinking
                            engine_actor.do_send(dispatcher::EngineStopThinking);
                            let engine_msg = dispatcher::EngineGetBestMove;
                            let debug_actor_opt = self.debug_actor_opt.clone();
                            engine_actor
                                .send(engine_msg)
                                .into_actor(self)
                                .map(
                                    move |result: Result<Option<TimestampedBitBoardMove>, _>,
                                          act,
                                          _ctx| {
                                        match result {
                                            Ok(Some(best_move)) => {
                                                if let Some(debug_actor) = &debug_actor_opt {
                                                    debug_actor.do_send(debug::AddMessage(
                                                        "Best move updated successfully"
                                                            .to_string(),
                                                    ));
                                                }
                                                act.ts_best_move_opt =
                                                    Some(best_move.to_ts_best_move());
                                            }
                                            Ok(None) => {
                                                if let Some(debug_actor) = &debug_actor_opt {
                                                    debug_actor.do_send(debug::AddMessage(
                                                        "No move found.".to_string(),
                                                    ));
                                                }
                                                act.ts_best_move_opt = None;
                                            }
                                            Err(e) => {
                                                if let Some(debug_actor) = &debug_actor_opt {
                                                    debug_actor.do_send(debug::AddMessage(
                                                        format!(
                                                            "Error sending message to engine: {:?}",
                                                            e
                                                        ),
                                                    ));
                                                }
                                                act.ts_best_move_opt = None;
                                            }
                                        }
                                    },
                                )
                                .wait(ctx); // Wait for the future to complete within the actor context
                        }
                        Err(err) => {
                            println!("Failed to retrieve engine actor: {:?}", err);
                            self.ts_best_move_opt = None;
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
    Btime(u64),                                                // Update clock for black
    BtimeInc(u64),                                             // Update increment clock for black
    CleanResources,                                            // Clean resources
    DepthFinite(u32),                                          // Set depth
    EngineStartThinking,                                       // Go command: start calculating
    EngineStopThinking,                                        // Stop command: retrieve best move
    InitPosition,                                              // Set starting position
    MaxTimePerMoveInMs(u32),                                   // Set maximum time per move
    SearchMoves(Vec<long_notation::LongAlgebricNotationMove>), // Focus on a list of moves for analysis
    SearchInfinite,                                            // Set infinite search
    UpdatePosition(String, fen::Position),                     // Set a new position
    ValidMoves {
        // Play moves from the current position
        moves: Vec<long_notation::LongAlgebricNotationMove>,
    },
    Wtime(u64),    // Update clock for white
    WtimeInc(u64), // Update increment clock for white
}

// Message to set the clocks in the Game actor
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetClocks {
    white_clock_actor_opt: Option<chessclock::ClockActor>,
    black_clock_actor_opt: Option<chessclock::ClockActor>,
}
#[cfg(test)]
impl SetClocks {
    pub fn new(
        white_clock_actor_opt: Option<chessclock::ClockActor>,
        black_clock_actor_opt: Option<chessclock::ClockActor>,
    ) -> Self {
        SetClocks {
            white_clock_actor_opt,
            black_clock_actor_opt,
        }
    }
}
impl Handler<SetClocks> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: SetClocks, _ctx: &mut Context<Self>) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
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

impl GameManager {
    fn play_moves(&mut self, valid_moves: Vec<LongAlgebricNotationMove>) -> Result<(), String> {
        let result: Option<Result<Vec<bitboard::BitBoardMove>, String>> = self
            .game_state_opt
            .as_mut()
            .map(|game_state: &mut GameState| {
                game_state.play_moves(
                    valid_moves,
                    &self.zobrist_table,
                    self.debug_actor_opt.clone(),
                )
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
    pub fn set_players(&mut self, players: player::Players) {
        self.players = players;
    }
}

fn async_clock_inc(debug: String, n_moves: u64, clock_actor: Addr<chessclock::Clock>) {
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

impl Handler<chessclock::TimeOut> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: chessclock::TimeOut, _ctx: &mut Context<Self>) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        if let Some(ref mut game_state) = &mut self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
            if game_state
                .check_insufficient_material_for_color(color.switch(), bit_boards_white_and_black)
            {
                game_state.set_end_game(game_state::EndGame::TimeOutDraw);
                println!("set end game TimeOutDraw");
            } else {
                game_state.set_end_game(game_state::EndGame::TimeOutLost(color));
                println!("set end game: TimeOutLost");
            }
        } else {
            panic!("A clock has been started but no position has been set.")
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Option<u64>")]
pub struct GetClockRemainingTime(square::Color);

#[cfg(test)]
impl GetClockRemainingTime {
    pub fn new(color: square::Color) -> Self {
        GetClockRemainingTime(color)
    }
}

impl Handler<GetClockRemainingTime> for GameManager {
    type Result = ResponseFuture<Option<u64>>;

    fn handle(&mut self, msg: GetClockRemainingTime, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
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

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetClockRemainingTime {
    color: square::Color,
    remaining_time: u64,
}

impl Handler<SetClockRemainingTime> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: SetClockRemainingTime, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
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

#[derive(Debug, Message)]
#[rtype(result = "()")]
struct StartOrSwitchClocks;

// Implementing a handler for starting the clocks
impl Handler<StartOrSwitchClocks> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: StartOrSwitchClocks, _ctx: &mut Context<Self>) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
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

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct GetBestMoveFromUci<R>
where
    R: uci::UciRead + 'static,
{
    uci_caller: Addr<uci::UciEntity<R>>,
}
impl<R> GetBestMoveFromUci<R>
where
    R: uci::UciRead + 'static,
{
    pub fn new(uci_caller: Addr<uci::UciEntity<R>>) -> Self {
        Self { uci_caller }
    }
}

impl<R> Handler<GetBestMoveFromUci<R>> for GameManager
where
    R: uci::UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: GetBestMoveFromUci<R>, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(
                "game_manager_actor receive GetBestMoveFromUci".to_string(),
            ));
        }
        let engine_still_thinking = false;
        let reply =
            uci::UciResult::DisplayBestMove(self.ts_best_move_opt.clone(), !engine_still_thinking);
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor send to uci entity: '{:?}'",
                reply
            )));
        }
        msg.uci_caller.do_send(reply);
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Option<TimestampedBestMove>")]
pub struct GetBestMove;

impl Handler<GetBestMove> for GameManager {
    type Result = Option<TimestampedBestMove>;

    fn handle(&mut self, msg: GetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        self.ts_best_move_opt.clone()
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct GetCurrentEngineAsync<R>
where
    R: uci::UciRead + 'static,
{
    uci_caller: Addr<uci::UciEntity<R>>,
}
impl<R> GetCurrentEngineAsync<R>
where
    R: uci::UciRead + 'static,
{
    pub fn new(uci_caller: Addr<uci::UciEntity<R>>) -> Self {
        Self { uci_caller }
    }
}
impl<R> Handler<GetCurrentEngineAsync<R>> for GameManager
where
    R: uci::UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: GetCurrentEngineAsync<R>, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(
                "game_manager_actor receive GetCurrentEngineAsync".to_string(),
            ));
        }
        if let Some(game_state) = &self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let engine_actor_opt = self.players.get_player_into(color).get_engine().cloned();
            if let Some(engine_actor) = engine_actor_opt {
                let reply = dispatcher::EngineGetIdAsync::new(msg.uci_caller.clone());
                engine_actor.do_send(reply);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct GetCurrentEngine;
impl Message for GetCurrentEngine {
    type Result = Option<Addr<dispatcher::EngineDispatcher>>;
}

impl Handler<GetCurrentEngine> for GameManager {
    type Result = Option<Addr<dispatcher::EngineDispatcher>>;

    fn handle(&mut self, msg: GetCurrentEngine, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
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

#[derive(Debug, Message)]
#[rtype(result = "Result<game_state::EndGame, ()>")]
pub struct GetEndGame;

impl Handler<GetEndGame> for GameManager {
    type Result = Result<game_state::EndGame, ()>;

    fn handle(&mut self, msg: GetEndGame, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        let end_game = match &self.game_state_opt {
            None => game_state::EndGame::None,
            Some(game_state) => game_state.end_game(),
        };
        Ok(end_game)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Option<GameState>")]
pub struct GetGameState;

impl Handler<GetGameState> for GameManager {
    type Result = Option<GameState>;

    fn handle(&mut self, msg: GetGameState, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        self.game_state().cloned()
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<parameters::Parameters, ()>")]
pub struct GetParameters;

impl Handler<GetParameters> for GameManager {
    type Result = Result<parameters::Parameters, ()>;

    fn handle(&mut self, msg: GetParameters, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        Ok(self.parameters.clone())
    }
}

#[derive(Debug, Clone)]
pub struct TimestampedBitBoardMove {
    best_move: bitboard::BitBoardMove,
    timestamp: chrono::DateTime<chrono::Utc>,
    engine_id: logic::EngineId,
}
impl TimestampedBitBoardMove {
    pub fn new(best_move: bitboard::BitBoardMove, engine_id: logic::EngineId) -> Self {
        Self {
            best_move,
            timestamp: chrono::Utc::now(),
            engine_id,
        }
    }
    pub fn to_ts_best_move(&self) -> TimestampedBestMove {
        let best_move = long_notation::LongAlgebricNotationMove::build_from_b_move(self.best_move);
        TimestampedBestMove::build(best_move, self.timestamp, self.engine_id.clone())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetBestMove(pub TimestampedBitBoardMove);
impl SetBestMove {
    pub fn new(best_move: bitboard::BitBoardMove, engine_id: logic::EngineId) -> Self {
        Self(TimestampedBitBoardMove::new(best_move, engine_id))
    }
    pub fn from_ts_move(ts_move: TimestampedBitBoardMove) -> Self {
        Self(ts_move)
    }
}

impl Handler<SetBestMove> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: SetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        let ts_best_move = msg.0;
        let ts_best_move_cast = TimestampedBestMove::build(
            long_notation::LongAlgebricNotationMove::build_from_b_move(ts_best_move.best_move),
            ts_best_move.timestamp,
            ts_best_move.engine_id,
        );
        let mut is_update = true;
        if let Some(ts_best_move) = &self.ts_best_move_opt {
            if ts_best_move.is_more_recent_best_move_than(&ts_best_move_cast) {
                is_update = false;
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(
                        "best move not updated because not more recent than the current one"
                            .to_string(),
                    ));
                }
            }
        }
        if is_update {
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage("best move updated".to_string()));
            }
            self.ts_best_move_opt = Some(ts_best_move_cast);
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), String>")]
pub struct PlayMoves {
    moves: Vec<LongAlgebricNotationMove>,
}
impl PlayMoves {
    pub fn new(moves: Vec<LongAlgebricNotationMove>) -> Self {
        Self { moves }
    }
}

impl Handler<PlayMoves> for GameManager {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PlayMoves, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        self.play_moves(msg.moves)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<History, ()>")]
pub struct GetHistory;

impl Handler<GetHistory> for GameManager {
    type Result = Result<History, ()>;

    fn handle(&mut self, msg: GetHistory, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        Ok(self.history().clone())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct StopActor;

impl Handler<StopActor> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: StopActor, ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        ctx.stop();
    }
}

#[cfg(test)]
pub async fn build_game_manager_actor(inputs: Vec<&str>) -> GameManagerActor {
    use crate::entity::engine::component::engine_dummy as dummy;
    use std::sync::Arc;

    let debug_actor_opt: Option<debug::DebugActor> = None;
    //let debug_actor_opt: Option<debug::DebugActor> = Some(debug::DebugEntity::new(true).start());
    let uci_reader = crate::uci::UciReadVecStringWrapper::new(&inputs);
    let mut game = GameManager::new(debug_actor_opt.clone());
    let engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
    let engine_player1_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
    let engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
    let engine_player2_dispatcher =
        dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone());
    let player1 = player::Player::Human {
        engine_opt: Some(engine_player1_dispatcher.start()),
    };
    let player2 = player::Player::Computer {
        engine: engine_player2_dispatcher.start(),
    };
    let players = player::Players::new(player1, player2);
    game.set_players(players);
    let game_manager_actor = GameManager::start(game);
    // set the position from uci command
    let uci_entity = uci::UciEntity::new(
        uci_reader,
        game_manager_actor.clone(),
        debug_actor_opt.clone(),
    );
    let uci_entity_actor = uci_entity.start();
    for _i in 0..inputs.len() {
        let _r = uci_entity_actor.send(uci::ReadUserInput).await;
    }
    actix::clock::sleep(std::time::Duration::from_millis(100)).await;
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
    actix::clock::sleep(std::time::Duration::from_millis(100)).await;
    game_manager_actor
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use actix::Actor;

    use super::GameManager;
    use crate::entity::clock::actor::chessclock;
    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::entity::engine::component::engine_dummy as dummy;
    use crate::entity::game::actor::game_manager;
    use crate::entity::game::component::{game_state, player, square};
    use crate::monitoring::debug;
    use crate::uci;
    use crate::ui::notation::fen::{self, EncodeUserInput};

    // FIXME: redudant with uci.rs test
    async fn get_game_state(
        game_manager_actor: &game_manager::GameManagerActor,
    ) -> Option<game_state::GameState> {
        let result_or_error = game_manager_actor.send(game_manager::GetGameState).await;
        result_or_error.unwrap()
    }

    #[actix::test]
    async fn test_game_capture_en_passant_valid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 e7e5 d5e6"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnbqkbnr/ppp2ppp/4P3/8/8/8/PPPP1PPP/RNBQKBNR b KQkq - 0 3";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_pawn_move_invalid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 e7e5 e4e5"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        assert_eq!(fen, fen_expected);
    }

    #[actix::test]
    async fn test_game_mat() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves e2e4 e7e5 f1c4 a7a6 d1f3 a6a5 f3f7"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _r = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Mat(square::Color::Black))
    }
    #[actix::test]
    async fn test_game_pat_white_first() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K w - - 0 1 moves h1g1"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _r = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_pat_black_first() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position fen k7/7R/1R6/8/8/8/8/7K b - - 0 1"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = game_manager::GameManager::start(game_manager::GameManager::new(
            debug_actor_opt.clone(),
        ));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _r = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Pat)
    }
    #[actix::test]
    async fn test_game_weird() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves d2d4 d7d5 b1c3 a7a6 c1f4 a6a5 d1d2 a5a4 e1c1 a4a3 h2h3 a3b2 c1b1 a8a2 h3h4 a2a1 b1b2"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        let mut is_error = false;
        for _i in 0..inputs.len() {
            let r = uci_entity_actor.send(uci::ReadUserInput).await;
            if r.is_err() {
                is_error = true;
            }
        }
        assert!(!is_error)
    }
    #[actix::test]
    async fn test_game_blocked_pawn_ckeck_invalid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 e7e5 a2a3 d8h4 f2f3"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnb1kbnr/pppp1ppp/8/4p3/4P2q/P7/1PPP1PPP/RNBQKBNR w KQkq - 1 3";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_block_ckeck() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some( debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1f3"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = game_manager::GameManager::start(game_manager::GameManager::new(
            debug_actor_opt.clone(),
        ));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnb1kbnr/ppp1pppp/8/4q3/8/P7/1PPP1PPP/RNBQKBNR w KQkq - 1 4";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_block_ckeck2() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves e2e4 d7d5 e4d5 d8d5 a2a3 d5e5 d1e2"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        let mut is_error = false;
        for _i in 0..inputs.len() {
            let r = uci_entity_actor.send(uci::ReadUserInput).await;
            if r.is_err() {
                is_error = true;
            }
        }
        assert!(!is_error)
    }
    #[actix::test]
    async fn test_game_escape() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let inputs = vec!["position startpos moves e2e4 c7c5 f1c4 d7d6 d1h5 a7a6 h5f7 e8d7"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        let fen_expected = "rnbq1bnr/1p1kpQpp/p2p4/2p5/2B1P3/8/PPPP1PPP/RNB1K1NR w KQ - 1 5";
        assert_eq!(fen, fen_expected);
    }
    #[actix::test]
    async fn test_game_king_close_to_king() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let fen_initial = "r7/8/8/4k3/8/4K3/8/7R w - - 0 1";
        let position = format!("position fen {} moves e3e4", fen_initial);
        let inputs = vec![position.as_str()];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_initial);
    }
    #[actix::test]
    async fn test_game_rule_insufficient_material() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position fen k7/8/8/8/8/8/8/7K b - - 0 1"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::InsufficientMaterial)
    }
    #[actix::test]
    async fn test_game_rule_50xmoves() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
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
        let inputs = vec![fen.as_str()];
        let game_manager_actor =
            game_manager::GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::NoPawnAndCapturex50)
    }

    #[actix::test]
    async fn test_game_3x_position() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        //let debug_actor_opt = Some(debug::DebugEntity::new(true).start());
        let moves = "h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!("position fen k7/8/r7/8/8/7R/8/7K w - - 0 1 moves {}", moves);
        let inputs = vec![fen.as_str()];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Repetition3x)
    }
    #[actix::test]
    async fn test_game_3x_position_with_pawn() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let moves = "h7h6 h1g1 a8b8 g1h1 b8a8 h1g1 a8b8 g1h1 b8a8";
        let fen = format!(
            "position fen k7/7p/r7/8/8/7R/8/7K b - - 0 1 moves {}",
            moves
        );
        let inputs = vec![fen.as_str()];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let game_manager_actor = GameManager::start(GameManager::new(debug_actor_opt.clone()));
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
        let end_game = game_manager_actor
            .send(game_manager::GetEndGame)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(end_game, game_state::EndGame::Repetition3x)
    }

    #[actix::test]
    async fn test_game_timeout_gameover() {
        let inputs = vec!["position startpos"];
        let game_manager_actor = game_manager::build_game_manager_actor(inputs).await;
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
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec!["position startpos moves e2e4 e7e5 g1f3 g8f6 f1c4"];
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let mut game_manager = super::GameManager::new(debug_actor_opt.clone());
        let engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player1_dispatcher =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
        let engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player2_dispatcher =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone());
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1_dispatcher.start()),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2_dispatcher.start(),
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager::GameManager::start(game_manager);
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
        let uci_entity = uci::UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(uci::ReadUserInput).await;
        }
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
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K w - - 0 1"];
        let game_manager_actor = game_manager::build_game_manager_actor(inputs).await;
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
        let inputs = vec!["position fen k7/7p/8/8/8/8/8/7K b - - 0 1"];
        let game_manager_actor = game_manager::build_game_manager_actor(inputs).await;
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
