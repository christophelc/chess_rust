pub mod handler_event;
pub mod handler_poll;
pub mod handler_read;
pub mod handler_uci;

use crate::{
    entity::uci::component::command,
    ui::notation::{
        fen::{self, EncodeUserInput},
        long_notation,
    },
};
use actix::{
    dev::ContextFutureSpawner, Actor, AsyncContext, Context, Handler, Message, WrapFuture,
};
use std::{
    error::Error,
    io::{self, Stdin, Stdout, Write},
    sync::{Arc, Mutex},
};

use crate::entity::engine::component::engine_logic as logic;
use crate::entity::game::actor::game_manager;
use crate::entity::uci::component::event;
use crate::monitoring::debug;

const POLLING_INTERVAL_MS: u64 = 50;

#[derive(Debug, PartialEq)]
pub enum StatePollingUciEntity {
    Pending,
    Polling,
}

pub struct UciEntity<R>
where
    R: UciRead + 'static,
{
    stdout: Stdout,
    state_polling: StatePollingUciEntity,
    uci_reader: R,
    game_manager_actor: game_manager::GameManagerActor,
    debug_actor_opt: Option<debug::DebugActor>,
}
impl<R> Actor for UciEntity<R>
where
    R: UciRead + 'static,
{
    type Context = Context<Self>;
}

impl<R> UciEntity<R>
where
    R: UciRead + 'static,
{
    pub fn new(
        uci_reader: R,
        game_manager_actor: game_manager::GameManagerActor,
        debug_actor_opt: Option<debug::DebugActor>,
    ) -> Self {
        Self {
            stdout: io::stdout(),
            state_polling: StatePollingUciEntity::Pending,
            uci_reader,
            game_manager_actor,
            debug_actor_opt,
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), io::Error>")]
pub struct DisplayEngineId(pub logic::EngineId);

impl<R: UciRead> Handler<DisplayEngineId> for UciEntity<R> {
    type Result = Result<(), io::Error>;

    fn handle(&mut self, msg: DisplayEngineId, _ctx: &mut Self::Context) -> Self::Result {
        let engine_id = msg.0;
        writeln!(self.stdout, "id name {}", engine_id.name())?;
        writeln!(self.stdout, "id author {}", engine_id.author())?;
        writeln!(self.stdout, "uciok")?;
        Ok(())
    }
}

pub trait UciRead: Unpin {
    fn uci_read(&mut self) -> Option<String>;
}
pub struct UciReadWrapper {
    stdin: Arc<Mutex<Stdin>>,
}
impl UciReadWrapper {
    pub fn new(stdin: Arc<Mutex<Stdin>>) -> Self {
        UciReadWrapper { stdin }
    }
}

impl UciRead for UciReadWrapper {
    fn uci_read(&mut self) -> Option<String> {
        let mut input = String::new();
        self.stdin
            .lock()
            .unwrap()
            .read_line(&mut input)
            .expect("Failed to read line");
        // useful for testing purpose
        let s = input.trim().to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

pub struct UciReadVecStringWrapper {
    idx: usize,
    inputs: Vec<String>,
}
impl UciReadVecStringWrapper {
    pub fn new(inputs: &[&str]) -> Self {
        let inputs_to_string = inputs.iter().map(|s| String::from(*s)).collect();
        UciReadVecStringWrapper {
            idx: 0,
            inputs: inputs_to_string,
        }
    }
}
impl UciRead for UciReadVecStringWrapper {
    fn uci_read(&mut self) -> Option<String> {
        if self.idx < self.inputs.len() {
            let result = self.inputs.get(self.idx).unwrap();
            self.idx += 1;
            Some(result.to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::entity::clock::actor::chessclock;
    use crate::entity::game::actor::game_manager;
    use crate::entity::game::component::square;
    use crate::entity::game::component::{game_state, parameters, player};
    use crate::ui::notation::fen::{self, EncodeUserInput};
    use actix::{Actor, Addr};
    use command::parser;
    use parser::InputParser;

    use crate::entity::engine::actor::engine_dispatcher as dispatcher;
    use crate::entity::engine::component::engine_dummy as dummy;

    // read all inputs and execute UCI commands
    async fn exec_inputs(
        uci_entity_actor: Addr<UciEntity<UciReadVecStringWrapper>>,
        inputs: Vec<&str>,
    ) {
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(handler_read::ReadUserInput).await;
        }
    }
    async fn init(input: &str) -> (game_manager::GameManagerActor, command::Command) {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let game_manager_actor = game_manager::GameManager::start(game_manager::GameManager::new(
            debug_actor_opt.clone(),
        ));
        let parser = InputParser::new(&input, game_manager_actor.clone());
        let command = parser.parse_input().expect("Invalid command");
        (game_manager_actor, command)
    }
    async fn get_game_state(
        game_manager_actor: &game_manager::GameManagerActor,
    ) -> Option<game_state::GameState> {
        let result_or_error = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await;
        result_or_error.unwrap()
    }

    #[actix::test]
    async fn test_uci_input_start_pos() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = "position startpos";
        let inputs = vec![input];
        let (game_manager_actor, _command) = init(input).await;
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(handler_read::ReadUserInput)
            .await
            .expect("Actix error")
            .unwrap();
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }

    #[actix::test]
    async fn test_uci_input_start_pos_with_moves() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = "position startpos moves e2e4 e7e5 g1f3";
        let inputs = vec![input];
        let (game_manager_actor, _command) = init(input).await;
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(handler_read::ReadUserInput)
            .await
            .expect("Actix error")
            .unwrap();
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }

    #[actix::test]
    async fn test_uci_input_fen_pos() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = format!("position fen {}", fen::FEN_START_POSITION);
        let inputs = vec![input.as_str()];
        let (game_manager_actor, _) = init(&input).await;
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(handler_read::ReadUserInput)
            .await
            .expect("Actix error")
            .unwrap();
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }
    #[actix::test]
    async fn test_uci_input_fen_pos_with_moves() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = format!(
            "position fen {} moves e2e4 e7e5 g1f3",
            fen::FEN_START_POSITION
        );
        let inputs = vec![input.as_str()];
        let (game_manager_actor, _command) = init(&input).await;
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(handler_read::ReadUserInput)
            .await
            .expect("Actix error")
            .unwrap();
        let game_opt = get_game_state(&game_manager_actor).await;
        assert!(game_opt.is_some());
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::Fen::encode(&game_opt.unwrap().bit_position().to())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }
    #[actix::test]
    async fn test_uci_input_fen_pos_with_moves_invalid() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = format!(
            "position fen {} moves e2e4 e7e5 g1f4",
            fen::FEN_START_POSITION
        );
        let inputs = vec![input.as_str()];
        let (game_manager_actor, _command) = init(&input).await;
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        exec_inputs(uci_entity_actor, inputs).await;
        // check the last move has not been played
        let game_state = game_manager_actor
            .send(game_manager::handler_game::GetGameState)
            .await
            .expect("actix error")
            .expect("empty game");
        let fen = fen::Fen::encode(&game_state.bit_position().to()).unwrap();
        let expected_fen = "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2";
        assert_eq!(fen, expected_fen)
    }
    #[actix::test]
    async fn test_uci_input_default_parameters() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = "position startpos";
        let inputs = vec![input];
        let (game_manager_actor, _command) = init(&input).await;
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        exec_inputs(uci_entity_actor, inputs).await;
        let parameters = game_manager_actor
            .send(game_manager::handler_game::GetParameters)
            .await
            .expect("mailbox error")
            .unwrap();
        let expected = parameters::Parameters::default();
        assert_eq!(parameters, expected)
    }

    #[actix::test]
    async fn test_uci_input_modified_parameters() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let inputs = vec![
            "position startpos",
            "go depth 3 movetime 5000 wtime 3600000 btime 3600001",
        ];
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
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
        // set clocks before executing UCI commands
        let white_clock_actor =
            chessclock::Clock::new("white", 3, 0, game_manager_actor.clone()).start();
        let black_clock_actor =
            chessclock::Clock::new("black", 3, 0, game_manager_actor.clone()).start();
        game_manager_actor.do_send(game_manager::handler_clock::SetClocks::new(
            Some(white_clock_actor),
            Some(black_clock_actor),
        ));
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        exec_inputs(uci_entity_actor, inputs).await;
        actix::clock::sleep(Duration::from_millis(100)).await;
        let parameters = game_manager_actor
            .send(game_manager::handler_game::GetParameters)
            .await
            .expect("Actix error")
            .unwrap();
        let expected = parameters::Parameters::new(
            Some(3),
            Some(5000),
            Some(3600000),
            Some(3600001),
            None,
            None,
            vec![],
        );
        assert_eq!(parameters, expected);
        // check wtime and btime
        let remaining_time_white = game_manager_actor
            .send(game_manager::handler_clock::GetClockRemainingTime::new(
                square::Color::White,
            ))
            .await
            .expect("actor error")
            .expect("Missing data");
        let remaining_time_black = game_manager_actor
            .send(game_manager::handler_clock::GetClockRemainingTime::new(
                square::Color::Black,
            ))
            .await
            .expect("actor error")
            .expect("Missing data");
        assert_eq!(remaining_time_white, 3600000);
        assert_eq!(remaining_time_black, 3600001);
    }

    #[actix::test]
    async fn test_uci_start_stop_think_engine() {
        let debug_actor = debug::DebugEntity::new(true).start();
        let debug_actor_opt = Some(debug_actor.clone());
        let inputs = vec!["position startpos", "go"];
        let uci_reader = UciReadVecStringWrapper::new(&inputs);
        let mut game_manager = game_manager::GameManager::new(debug_actor_opt.clone());
        let engine_player1 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player1_dispatcher_actor =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone())
                .start();
        let engine_player2 = dummy::EngineDummy::new(debug_actor_opt.clone());
        let engine_player2_dispatcher_actor =
            dispatcher::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone())
                .start();
        let player1 = player::Player::Human {
            engine_opt: Some(engine_player1_dispatcher_actor.clone()),
        };
        let player2 = player::Player::Computer {
            engine: engine_player2_dispatcher_actor.clone(),
        };
        let players = player::Players::new(player1, player2);
        game_manager.set_players(players);
        let game_manager_actor = game_manager::GameManager::start(game_manager);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        exec_inputs(uci_entity_actor, inputs).await;
        actix::clock::sleep(Duration::from_millis(1000)).await;
        let debug_msgs = debug_actor
            .send(debug::ShowAllMessages)
            .await
            .expect("Actix error");
        let engine_status = engine_player1_dispatcher_actor
            .send(dispatcher::handler_engine::EngineGetStatus)
            .await
            .expect("Actix error")
            .unwrap();
        let engine_is_thinking = false;
        let expected = dispatcher::EngineStatus::new(engine_is_thinking);
        let _ = game_manager_actor
            .send(game_manager::handler_uci_command::UciCommand::CleanResources)
            .await
            .unwrap();
        assert_eq!(engine_status, expected);
        let debug_start_thinking: Option<String> = debug_msgs
            .into_iter()
            .find(|el| el.contains("EngineStartThinkin"));
        assert!(debug_start_thinking.is_some());
    }
}

#[derive(Debug)]
pub struct HandleEventError {
    event: event::Event,
    error: String,
}
impl HandleEventError {
    pub fn new(event: event::Event, error: String) -> Self {
        HandleEventError { event, error }
    }
}
impl Error for HandleEventError {}
impl std::fmt::Display for HandleEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "HandleEvent error for event {:?}. The error is: {}",
            self.event, self.error
        )
    }
}
impl<R: UciRead> Handler<event::Event> for UciEntity<R> {
    type Result = ();

    fn handle(&mut self, msg: event::Event, ctx: &mut Self::Context) -> Self::Result {
        let actor_self = ctx.address();
        match msg {
            event::Event::Write(s) => {
                writeln!(self.stdout, "{}", s).unwrap();
                self.stdout.flush().unwrap();
            }
            event::Event::WriteDebug(s) => {
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(format!("Uci actor event: {}", s)))
                }
            }
            event::Event::DebugMode(debug_actor_opt) => {
                self.debug_actor_opt = debug_actor_opt;
            }
            event::Event::StartPos => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::InitPosition);
            }
            event::Event::Fen(fen) => {
                let position = fen::Fen::decode(&fen).expect("Failed to decode FEN");
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::UpdatePosition(
                        fen.to_string(),
                        position,
                    ),
                );
            }
            // Go command
            event::Event::Depth(depth) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::DepthFinite(depth),
                );
            }
            event::Event::SearchInfinite => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::SearchInfinite);
            }
            event::Event::MaxTimePerMoveInMs(time) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::MaxTimePerMoveInMs(time),
                );
            }
            ref event @ event::Event::Moves(ref moves) => match moves_validation(moves) {
                Ok(valid_moves) => {
                    self.game_manager_actor.do_send(
                        game_manager::handler_uci_command::UciCommand::ValidMoves {
                            moves: valid_moves,
                        },
                    );
                }
                Err(err) => {
                    actor_self.do_send(handler_uci::UciResult::Err(HandleEventError::new(
                        event.clone(),
                        err,
                    )));
                }
            },
            event::Event::Wtime(wtime) => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::Wtime(wtime));
            }
            event::Event::Btime(btime) => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::Btime(btime));
            }
            event::Event::WtimeInc(wtime_inc) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::WtimeInc(wtime_inc),
                );
            }
            event::Event::BtimeInc(btime_inc) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::BtimeInc(btime_inc),
                );
            }
            ref event @ event::Event::SearchMoves(ref search_moves) => {
                match moves_validation(search_moves) {
                    Ok(valid_moves) => {
                        self.game_manager_actor.do_send(
                            game_manager::handler_uci_command::UciCommand::SearchMoves(valid_moves),
                        );
                    }
                    Err(err) => {
                        actor_self.do_send(handler_uci::UciResult::Err(HandleEventError::new(
                            event.clone(),
                            err,
                        )));
                    }
                }
            }
            event::Event::StartEngine => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::EngineStartThinking);
                self.state_polling = StatePollingUciEntity::Polling;
                ctx.address().do_send(handler_poll::PollBestMove);
            }
            event::Event::StopEngine => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::EngineStopThinking);
            }
            event::Event::Quit => {
                actor_self.do_send(handler_uci::UciResult::Quit);
            }
        }
    }
}

pub fn moves_validation(
    moves: &Vec<String>,
) -> Result<Vec<long_notation::LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<long_notation::LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match long_notation::LongAlgebricNotationMove::build_from_str(m) {
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

impl<R> Handler<command::Command> for UciEntity<R>
where
    R: UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: command::Command, ctx: &mut Self::Context) -> Self::Result {
        let mut events: Vec<event::Event> = vec![];
        match msg {
            command::Command::Wait100ms => {
                use tokio::time::{sleep, Duration};

                events.push(event::Event::WriteDebug("waiting 100ms".to_string()));
                sleep(Duration::from_millis(100)).into_actor(self).wait(ctx);
            }
            command::Command::Uci(game_manager_actor) => {
                let uci_caller = ctx.address();
                let msg = game_manager::handler_engine::GetCurrentEngineAsync::new(uci_caller);
                game_manager_actor.do_send(msg);
            }
            command::Command::Ignore => {}
            command::Command::IsReady => events.push(event::Event::Write("readyok".to_string())),
            command::Command::DebugMode(is_debug) => {
                events.push(event::Event::WriteDebug(format!(
                    "debug mode set to {}",
                    is_debug
                )));
                events.push(event::Event::DebugMode(self.debug_actor_opt.clone()));
            }
            command::Command::NewGame => {
                events.push(event::Event::StartPos);
                // TODO: reset btime, wtime ?
            }
            command::Command::Position(pos) => {
                events.push(event::Event::WriteDebug("Position received".to_string()));
                if pos.startpos() {
                    events.push(event::Event::WriteDebug(
                        "Set board to starting position.".to_string(),
                    ));
                    events.push(event::Event::StartPos);
                } else if let Some(fen_str) = pos.fen() {
                    events.push(event::Event::WriteDebug(
                        format!("Set board to FEN: {}", fen_str).to_string(),
                    ));
                    events.push(event::Event::Fen(fen_str));
                }
                if !pos.moves().is_empty() {
                    events.push(event::Event::WriteDebug(
                        format!("Moves played: {:?}", pos.moves()).to_string(),
                    ));
                    events.push(event::Event::Moves(pos.moves().clone()));
                }
            }
            command::Command::Go(go) => {
                if let Some(d) = go.depth() {
                    events.push(event::Event::WriteDebug(
                        format!("Searching to depth: {}", d).to_string(),
                    ));
                    events.push(event::Event::Depth(d));
                }
                if let Some(time) = go.movetime() {
                    events.push(event::Event::WriteDebug(
                        format!("Max time for move: {} ms", time).to_string(),
                    ));
                    events.push(event::Event::MaxTimePerMoveInMs(time));
                }
                if go.infinite() {
                    events.push(event::Event::WriteDebug(
                        "Searching indefinitely...".to_string(),
                    ));
                    events.push(event::Event::SearchInfinite);
                }
                if let Some(wtime) = go.wtime() {
                    events.push(event::Event::WriteDebug(
                        format!("White time left: {} ms", wtime).to_string(),
                    ));
                    events.push(event::Event::Wtime(wtime));
                }
                if let Some(btime) = go.btime() {
                    events.push(event::Event::WriteDebug(
                        format!("Black time left: {} ms", btime).to_string(),
                    ));
                    events.push(event::Event::Btime(btime));
                }
                if let Some(wtime_inc) = go.wtime_inc() {
                    events.push(event::Event::WriteDebug(
                        format!("White time inc: {} ms", wtime_inc).to_string(),
                    ));
                    events.push(event::Event::WtimeInc(wtime_inc));
                }
                if let Some(btime_inc) = go.btime_inc() {
                    events.push(event::Event::WriteDebug(
                        format!("Black time left: {} ms", btime_inc).to_string(),
                    ));
                    events.push(event::Event::BtimeInc(btime_inc));
                }
                if !go.search_moves().is_empty() {
                    events.push(event::Event::WriteDebug(format!(
                        "Limit search to these moves: {:?}",
                        go.search_moves()
                    )));
                    events.push(event::Event::SearchMoves(go.search_moves().clone()));
                }
                events.push(event::Event::StartEngine)
            }
            command::Command::Stop => {
                events.push(event::Event::WriteDebug("Stopping search.".to_string()));
                events.push(event::Event::StopEngine);
            }
            command::Command::Quit => {
                events.push(event::Event::WriteDebug(
                    "Stopping search (Quit).".to_string(),
                ));
                events.push(event::Event::StopEngine);
                events.push(event::Event::WriteDebug("Exiting engine".to_string()));
                events.push(event::Event::Quit);
            }
        }
        let msg = handler_event::ProcessEvents(events);
        ctx.address().do_send(msg);
    }
}
