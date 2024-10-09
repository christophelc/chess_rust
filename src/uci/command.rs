pub mod parser;
use actix::dev::ContextFutureSpawner;
use actix::{Addr, AsyncContext, Handler, Message, WrapFuture};

use crate::game::game_manager;
use crate::uci;

use super::event;
use std::error::Error;

#[derive(Debug)]
pub struct CommandError {
    command: String,
}
impl CommandError {
    pub fn new(command: String) -> Self {
        CommandError { command }
    }
}
impl Error for CommandError {}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Command error for command: '{}'", self.command)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub enum Command {
    DebugMode(bool),
    Go(GoStruct),
    Ignore,  // do nothing
    IsReady, // "isready" command, no additional data needed
    NewGame,
    Position(PositionStruct),
    Quit,                                 // "quit" command to exit the engine
    Stop,                                 // "stop" command to stop search
    Uci(Addr<game_manager::GameManager>), // "uci" command, no additional data needed
    Wait100ms,                            // for test purpose
}

impl<R> Handler<Command> for super::UciEntity<R>
where
    R: uci::UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: Command, ctx: &mut Self::Context) -> Self::Result {
        let mut events: Vec<event::Event> = vec![];
        match msg {
            Command::Wait100ms => {
                use tokio::time::{sleep, Duration};

                events.push(event::Event::WriteDebug(format!("waiting 100ms")));
                sleep(Duration::from_millis(100)).into_actor(self).wait(ctx);
            }
            Command::Uci(game_manager_actor) => {
                let uci_caller = ctx.address();
                let msg = game_manager::GetCurrentEngineAsync::new(uci_caller);
                game_manager_actor.do_send(msg);
            }
            Command::Ignore => {}
            Command::IsReady => events.push(event::Event::Write("readyok".to_string())),
            Command::DebugMode(is_debug) => {
                events.push(event::Event::WriteDebug(format!(
                    "debug mode set to {}",
                    is_debug
                )));
                events.push(event::Event::DebugMode(self.debug_actor_opt.clone()));
            }
            Command::NewGame => {
                events.push(event::Event::StartPos);
                // TODO: reset btime, wtime ?
            }
            Command::Position(pos) => {
                events.push(event::Event::WriteDebug("Position received".to_string()));
                if pos.startpos {
                    events.push(event::Event::WriteDebug(
                        "Set board to starting position.".to_string(),
                    ));
                    events.push(event::Event::StartPos);
                } else if let Some(fen_str) = pos.fen.clone() {
                    events.push(event::Event::WriteDebug(
                        format!("Set board to FEN: {}", fen_str).to_string(),
                    ));
                    events.push(event::Event::Fen(fen_str));
                }
                if !pos.moves.is_empty() {
                    events.push(event::Event::WriteDebug(
                        format!("Moves played: {:?}", pos.moves).to_string(),
                    ));
                    events.push(event::Event::Moves(pos.moves.clone()));
                }
            }
            Command::Go(go) => {
                if let Some(d) = go.depth {
                    events.push(event::Event::WriteDebug(
                        format!("Searching to depth: {}", d).to_string(),
                    ));
                    events.push(event::Event::Depth(d));
                }
                if let Some(time) = go.movetime {
                    events.push(event::Event::WriteDebug(
                        format!("Max time for move: {} ms", time).to_string(),
                    ));
                    events.push(event::Event::MaxTimePerMoveInMs(time));
                }
                if go.infinite {
                    events.push(event::Event::WriteDebug(
                        "Searching indefinitely...".to_string(),
                    ));
                    events.push(event::Event::SearchInfinite);
                }
                if let Some(wtime) = go.wtime {
                    events.push(event::Event::WriteDebug(
                        format!("White time left: {} ms", wtime).to_string(),
                    ));
                    events.push(event::Event::Wtime(wtime));
                }
                if let Some(btime) = go.btime {
                    events.push(event::Event::WriteDebug(
                        format!("Black time left: {} ms", btime).to_string(),
                    ));
                    events.push(event::Event::Btime(btime));
                }
                if let Some(wtime_inc) = go.wtime_inc {
                    events.push(event::Event::WriteDebug(
                        format!("White time inc: {} ms", wtime_inc).to_string(),
                    ));
                    events.push(event::Event::WtimeInc(wtime_inc));
                }
                if let Some(btime_inc) = go.btime_inc {
                    events.push(event::Event::WriteDebug(
                        format!("Black time left: {} ms", btime_inc).to_string(),
                    ));
                    events.push(event::Event::BtimeInc(btime_inc));
                }
                if !go.search_moves.is_empty() {
                    events.push(event::Event::WriteDebug(format!(
                        "Limit search to these moves: {:?}",
                        go.search_moves
                    )));
                    events.push(event::Event::SearchMoves(go.search_moves.clone()));
                }
                events.push(event::Event::StartEngine)
            }
            Command::Stop => {
                events.push(event::Event::WriteDebug("Stopping search.".to_string()));
                events.push(event::Event::StopEngine);
            }
            Command::Quit => {
                events.push(event::Event::WriteDebug(
                    "Stopping search (Quit).".to_string(),
                ));
                events.push(event::Event::StopEngine);
                events.push(event::Event::WriteDebug("Exiting engine".to_string()));
                events.push(event::Event::Quit);
            }
        }
        let msg = uci::ProcessEvents(events);
        ctx.address().do_send(msg);
    }
}

#[derive(Debug, Default)]
pub struct PositionStruct {
    // "position" command, with optional FEN and moves
    startpos: bool,      // `true` if the starting position is requested
    fen: Option<String>, // The FEN string, if specified (None if using startpos)
    moves: Vec<String>,  // A list of moves played after the position
}

#[derive(Debug, Default)]
pub struct GoStruct {
    // "go" command, with search parameters
    depth: Option<u32>,        // Optional depth to search
    movetime: Option<u32>,     // Optional maximum time for the move (in ms)
    infinite: bool,            // If true, search indefinitely until told to stop
    wtime: Option<u64>,        // White time left,
    btime: Option<u64>,        // Black time left
    wtime_inc: Option<u64>,    // White time increment per move
    btime_inc: Option<u64>,    // Black time increment per move
    search_moves: Vec<String>, // Restrict search to this moves only
}
