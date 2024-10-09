pub mod command;
pub mod event;
pub mod notation;
use crate::game::{
    engine,
    game_manager::{self, GetBestMoveFromUci},
    monitoring::debug,
};
use actix::{
    dev::ContextFutureSpawner, Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message,
    WrapFuture,
};
use command::parser;
use std::{
    io::{self, Stdin, Stdout, Write},
    sync::{Arc, Mutex},
    time::Duration,
};

const POLLING_INTERVAL_MS: u64 = 50;

#[derive(Message)]
#[rtype(result = "()")]
struct PollBestMove;

// TODO: Uci is in charge of polling game_engine_actor for best move each 100ms
// Handle polling requests from UCI actor
impl<R> Handler<PollBestMove> for UciEntity<R>
where
    R: UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, _msg: PollBestMove, ctx: &mut Self::Context) -> Self::Result {
        self.game_manager_actor
            .do_send(game_manager::GetBestMoveFromUci::new(ctx.address()));
        let debug_actor_opt = self.debug_actor_opt.clone();
        if self.state_polling == StatePollingUciEntity::Polling {
            ctx.run_later(
                Duration::from_millis(POLLING_INTERVAL_MS),
                move |actor, ctx| {
                    if let Some(debug_actor) = debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(
                            "UciEntity polling Game Manager to get best move...".to_string(),
                        ));
                    }
                    actor
                        .game_manager_actor
                        .do_send(GetBestMoveFromUci::new(ctx.address().clone()));
                },
            );
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub enum UciResult {
    Quit,
    DisplayBestMove(Option<game_manager::TimestampedBestMove>, bool), // maybe move, display in uci ui 'bestmove ...': bool
    Err(event::HandleEventError),
}

#[derive(Debug, PartialEq)]
enum StatePollingUciEntity {
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
        game_manager_actor: Addr<game_manager::GameManager>,
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

impl<R: UciRead> Handler<UciResult> for UciEntity<R> {
    type Result = ();

    fn handle(&mut self, msg: UciResult, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            UciResult::DisplayBestMove(timestamped_best_move_opt, is_show) => {
                if let Some(timestamped_best_move) = timestamped_best_move_opt {
                    // TODO: compare best move timestamp ? We could imagine competition between engine of different type searching for the best move
                    let msg_best_move =
                        format!("bestmove {}", timestamped_best_move.best_move().cast());
                    let msg_ts = format!("timestamp: {}", timestamped_best_move.timestamp());
                    let msg_origin = format!("origin: {:?}", timestamped_best_move.origin());
                    let msg = vec![msg_best_move, msg_ts, msg_origin].join(", ");
                    if let Some(debug_actor) = &self.debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(msg.to_string()));
                    }
                    if is_show {
                        let _ = writeln!(self.stdout, "{}", msg);
                        self.stdout.flush().unwrap();
                    }
                }
                self.state_polling = StatePollingUciEntity::Pending;
            }
            UciResult::Err(err) => {
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(err.to_string()));
                }
            }
            UciResult::Quit => {
                self.game_manager_actor.do_send(game_manager::StopActor);
                ctx.stop();
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ProcessEvents(pub Vec<event::Event>);

impl<R: UciRead> Handler<ProcessEvents> for UciEntity<R> {
    type Result = ();

    fn handle(&mut self, msg: ProcessEvents, ctx: &mut Self::Context) -> Self::Result {
        let events = msg.0;

        let addr = ctx.address();

        // Spawn a future within the actor context
        async move {
            for event in events {
                // Send the event and await its result
                let result = addr.send(event).await;

                match result {
                    Ok(_) => {
                        // Handle successful result
                    }
                    Err(e) => {
                        // Handle error
                        println!("Failed to send event: {:?}", e);
                    }
                }
            }
        }
        .into_actor(self) // Converts the future to an Actix-compatible future
        .spawn(ctx); // Spawns the future in the actor's context
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), io::Error>")]
pub struct DisplayEngineId(pub engine::EngineId);

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

#[derive(Message)]
#[rtype(result = "Result<(), Vec<String>> ")]
pub struct ReadUserInput;

impl<R: UciRead> Handler<ReadUserInput> for UciEntity<R> {
    type Result = Result<(), Vec<String>>;

    fn handle(&mut self, _msg: ReadUserInput, ctx: &mut Self::Context) -> Self::Result {
        let mut errors: Vec<String> = vec![];
        if let Some(input) = self.uci_reader.uci_read() {
            let parser = parser::InputParser::new(&input, self.game_manager_actor.clone());
            let command_or_error = parser.parse_input();
            match command_or_error {
                Ok(command) => {
                    if let Some(debug_actor) = &self.debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(format!(
                            "input '{}' send as coomand '{:?}' to game_manager_actor",
                            input, command
                        )));
                    }
                    ctx.address().do_send(command);
                }
                Err(err) => errors.push(err.to_string()),
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!("Errors: {}", errors.join("\n"))));
            }
            Err(errors)
        }
    }
}

pub trait UciRead: Unpin {
    fn uci_read(&mut self) -> Option<String>;
}
pub struct UciReadWrapper {
    stdin: Arc<Mutex<Stdin>>,
}
impl<'a> UciReadWrapper {
    pub fn new(stdin: Arc<Mutex<Stdin>>) -> Self {
        UciReadWrapper { stdin }
    }
}

impl<'a> UciRead for UciReadWrapper {
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
impl<'a> UciReadVecStringWrapper {
    pub fn new(inputs: &Vec<&str>) -> Self {
        let inputs_to_string = inputs.into_iter().map(|s| String::from(*s)).collect();
        UciReadVecStringWrapper {
            idx: 0,
            inputs: inputs_to_string,
        }
    }
}
impl<'a> UciRead for UciReadVecStringWrapper {
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
    use super::*;
    use crate::{
        board::{
            fen::{self, EncodeUserInput},
            square,
        },
        game, uci,
    };
    use actix::Actor;
    use game::{game_manager, game_state, parameters, player};
    use parser::InputParser;

    // read all inputs and execute UCI commands
    async fn exec_inputs(
        uci_entity_actor: Addr<UciEntity<UciReadVecStringWrapper>>,
        inputs: Vec<&str>,
    ) {
        for _i in 0..inputs.len() {
            let _ = uci_entity_actor.send(ReadUserInput).await;
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
        let result_or_error = game_manager_actor.send(game_manager::GetGameState).await;
        result_or_error.unwrap()
    }

    #[actix::test]
    async fn test_uci_input_start_pos() {
        let debug_actor_opt: Option<debug::DebugActor> = None;
        let input = "position startpos";
        let inputs = vec![input];
        let (game_manager_actor, _command) = init(input).await;
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(uci::ReadUserInput)
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
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(uci::ReadUserInput)
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
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(uci::ReadUserInput)
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
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_actor = uci_entity.start();
        uci_actor
            .send(uci::ReadUserInput)
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
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        exec_inputs(uci_entity_actor, inputs).await;
        // check the last move has not been played
        let game_state = game_manager_actor
            .send(game_manager::GetGameState)
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
        let uci_reader = uci::UciReadVecStringWrapper::new(&inputs);
        let uci_entity = UciEntity::new(
            uci_reader,
            game_manager_actor.clone(),
            debug_actor_opt.clone(),
        );
        let uci_entity_actor = uci_entity.start();
        exec_inputs(uci_entity_actor, inputs).await;
        let parameters = game_manager_actor
            .send(game_manager::GetParameters)
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
        let engine_player1 = engine::EngineDummy::new(debug_actor_opt.clone());
        let engine_player1_dispatcher =
            engine::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone());
        let engine_player2 = engine::EngineDummy::new(debug_actor_opt.clone());
        let engine_player2_dispatcher =
            engine::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone());
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
            game::chessclock::Clock::new("white", 3, 0, game_manager_actor.clone()).start();
        let black_clock_actor =
            game::chessclock::Clock::new("black", 3, 0, game_manager_actor.clone()).start();
        game_manager_actor.do_send(game_manager::SetClocks::new(
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
            .send(game_manager::GetParameters)
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
            .send(game_manager::GetClockRemainingTime::new(
                square::Color::White,
            ))
            .await
            .expect("actor error")
            .expect("Missing data");
        let remaining_time_black = game_manager_actor
            .send(game_manager::GetClockRemainingTime::new(
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
        let engine_player1 = engine::EngineDummy::new(debug_actor_opt.clone());
        let engine_player1_dispatcher_actor =
            engine::EngineDispatcher::new(Arc::new(engine_player1), debug_actor_opt.clone())
                .start();
        let engine_player2 = engine::EngineDummy::new(debug_actor_opt.clone());
        let engine_player2_dispatcher_actor =
            engine::EngineDispatcher::new(Arc::new(engine_player2), debug_actor_opt.clone())
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
            .send(engine::EngineGetStatus)
            .await
            .expect("Actix error")
            .unwrap();
        let engine_is_thinking = false;
        let engine_is_running = true;
        let expected = engine::EngineStatus::new(engine_is_thinking, engine_is_running);
        let _ = game_manager_actor
            .send(game_manager::UciCommand::CleanResources)
            .await
            .unwrap();
        assert_eq!(engine_status, expected);
        let debug_start_thinking: Option<String> = debug_msgs
            .into_iter()
            .find(|el| el.contains("EngineStartThinkin"));
        assert!(debug_start_thinking.is_some());
    }
}
