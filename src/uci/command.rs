pub mod parser;
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
    IsReady, // "isready" command, no additional data needed
    Position(PositionStruct),
    Go(GoStruct),
    Stop, // "stop" command to stop search
    Quit, // "quit" command to exit the engine
}
impl Command {
    pub fn handle_command(&self) -> Vec<event::Event> {
        let mut events: Vec<event::Event> = vec![];
        match self {
            Command::Uci => events.extend(vec![
                event::Event::Write("id name RandomEngine".to_string()),
                event::Event::Write("id author Christophe Le Cam".to_string()),
                event::Event::Write("uciok".to_string()),
            ]),
            Command::IsReady => events.push(event::Event::Write("readyok".to_string())),
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
                        format!("Time for move: {} ms", time).to_string(),
                    ));
                    events.push(event::Event::TimePerMoveInMs(time));
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
    search_moves: Vec<String>, // Restrict search to this moves only
}
