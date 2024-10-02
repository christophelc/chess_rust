pub mod parser;
use crate::game::{self, engine};

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

#[derive(Debug)]
pub enum Command {
    Uci,     // "uci" command, no additional data needed
    Ignore,  // do nothing
    IsReady, // "isready" command, no additional data needed
    Position(PositionStruct),
    NewGame,
    Go(GoStruct),
    Stop, // "stop" command to stop search
    Quit, // "quit" command to exit the engine
}
impl Command {
    pub async fn handle_command<T: engine::EngineActor>(
        &self,
        game_actor: &game::GameActor<T>,
    ) -> Vec<event::Event> {
        let mut events: Vec<event::Event> = vec![];
        match self {
            Command::Uci => {
                let msg = game::GetCurrentEngine::default();
                let result = game_actor.send(msg).await;
                if let Ok(Some(engine_actor)) = result {
                    let engine_id_opt = engine_actor.send(engine::EngineGetId::default()).await;
                    if let Ok(Some(engine_id)) = engine_id_opt {
                        events.extend(vec![
                            event::Event::Write(format!("id name {}", engine_id.name())),
                            event::Event::Write(format!("id author {}", engine_id.author())),
                            event::Event::Write("uciok".to_string()),
                        ]);
                    }
                }
            }
            Command::Ignore => {}
            Command::IsReady => events.push(event::Event::Write("readyok".to_string())),
            Command::NewGame => {
                events.push(event::Event::StartPos);
                // TODO: reset btime, wtime ?
            }
            Command::Position(pos) => {
                events.push(event::Event::Write("Position received".to_string()));
                if pos.startpos {
                    events.push(event::Event::Write(
                        "Set board to starting position.".to_string(),
                    ));
                    events.push(event::Event::StartPos);
                } else if let Some(fen_str) = pos.fen.clone() {
                    events.push(event::Event::Write(
                        format!("Set board to FEN: {}", fen_str).to_string(),
                    ));
                    events.push(event::Event::Fen(fen_str));
                }
                if !pos.moves.is_empty() {
                    events.push(event::Event::Write(
                        format!("Moves played: {:?}", pos.moves).to_string(),
                    ));
                    events.push(event::Event::Moves(pos.moves.clone()));
                }
            }
            Command::Go(go) => {
                if let Some(d) = go.depth {
                    events.push(event::Event::Write(
                        format!("Searching to depth: {}", d).to_string(),
                    ));
                    events.push(event::Event::Depth(d));
                }
                if let Some(time) = go.movetime {
                    events.push(event::Event::Write(
                        format!("Max time for move: {} ms", time).to_string(),
                    ));
                    events.push(event::Event::MaxTimePerMoveInMs(time));
                }
                if go.infinite {
                    events.push(event::Event::Write("Searching indefinitely...".to_string()));
                    events.push(event::Event::SearchInfinite);
                }
                if let Some(wtime) = go.wtime {
                    events.push(event::Event::Write(
                        format!("White time left: {} ms", wtime).to_string(),
                    ));
                    events.push(event::Event::Wtime(wtime));
                }
                if let Some(btime) = go.btime {
                    events.push(event::Event::Write(
                        format!("Black time left: {} ms", btime).to_string(),
                    ));
                    events.push(event::Event::Btime(btime));
                }
                if let Some(wtime_inc) = go.wtime_inc {
                    events.push(event::Event::Write(
                        format!("White time inc: {} ms", wtime_inc).to_string(),
                    ));
                    events.push(event::Event::WtimeInc(wtime_inc));
                }
                if let Some(btime_inc) = go.btime_inc {
                    events.push(event::Event::Write(
                        format!("Black time left: {} ms", btime_inc).to_string(),
                    ));
                    events.push(event::Event::Btime(btime_inc));
                }
                if !go.search_moves.is_empty() {
                    events.push(event::Event::Write(format!(
                        "Limit search to these moves: {:?}",
                        go.search_moves
                    )));
                    events.push(event::Event::SearchMoves(go.search_moves.clone()));
                }
                events.push(event::Event::StartEngine)
            }
            Command::Stop => {
                events.push(event::Event::Write("Stopping search.".to_string()));
                events.push(event::Event::StopEngine);
            }
            Command::Quit => {
                events.push(event::Event::Write("Stopping search.".to_string()));
                events.push(event::Event::StopEngine);
                events.push(event::Event::Write("Exiting engine".to_string()));
                events.push(event::Event::Quit);
            }
        }
        events
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
