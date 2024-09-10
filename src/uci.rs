use std::error::Error;
use std::fmt::Display;
use std::io::{self, stdout, BufRead, Stdin, Stdout, Write};

use crate::board::fen::{self, EncodeUserInput, Position};

pub enum Command {
    Uci,     // "uci" command, no additional data needed
    IsReady, // "isready" command, no additional data needed
    Position(PositionStruct),
    Go(GoStruct),
    Stop, // "stop" command to stop search
    Quit, // "quit" command to exit the engine
}
struct PositionStruct {
    // "position" command, with optional FEN and moves
    startpos: bool,      // `true` if the starting position is requested
    fen: Option<String>, // The FEN string, if specified (None if using startpos)
    moves: Vec<String>,  // A list of moves played after the position
}
struct GoStruct {
    // "go" command, with search parameters
    depth: Option<u32>,    // Optional depth to search
    movetime: Option<u32>, // Optional maximum time for the move (in ms)
    infinite: bool,        // If true, search indefinitely until told to stop
    wtime: Option<u64>,
    btime: Option<u64>,
    search_moves: Vec<String>,
}

pub struct LongAlgebricNotationMove {
    start: u8,
    end: u8,
}
impl LongAlgebricNotationMove {
    pub fn new(start: u8, end: u8) -> Self {
        LongAlgebricNotationMove { start, end }
    }
    pub fn build_from_str(move_str: String) -> Result<Self, String> {
        let mut result = Err(format!("Invalid move: {}", move_str));
        if move_str.len() == 4 {
            let from_square = &move_str[0..2]; // First two characters (e.g., "e2")
            let to_square = &move_str[2..4]; // Last two characters (e.g., "e4")
            let from_index = square_to_index(from_square);
            let to_index = square_to_index(to_square);
            if from_index < 64 && to_index < 64 {
                result = Ok(LongAlgebricNotationMove {
                    start: from_index,
                    end: to_index,
                });
            }
        }
        result
    }
    pub fn cast(&self) -> String {
        format!(
            "{}{}",
            index_to_string(self.start),
            index_to_string(self.end)
        )
    }
}
fn index_to_string(index: u8) -> String {
    assert!(index < 64, "index '{}' should be < 64", index);
    let row = index / 8;
    let col = index % 8;
    format!("{}{}", col, row)
}
fn col_as_char(col: u8) -> char {
    (b'a' + col) as char
}

fn square_to_index(square: &str) -> u8 {
    let col = square.chars().nth(0).unwrap() as u8 - 'a' as u8; // file 'a'-'h' -> 0-7
    let row = square.chars().nth(1).unwrap().to_digit(10).unwrap() as u8 - 1; // rank '1'-'8' -> 0-7
    (row * 8) + col
}

#[derive(Debug, Clone)]
pub enum Event {
    Write(String),
    StartPos,
    Fen(String),
    Moves(Vec<String>),
    Depth(u32),
    TimePerMoveInMs(u32),
    SearchInfinite,
    Wtime(u64),
    Btime(u64),
    SearchMoves(Vec<String>),
    Stop,
    Quit,
}
pub struct Configuration {
    opt_depth: Option<u32>,
    opt_time_per_move_in_ms: Option<u32>,
    opt_wtime: Option<u64>,
    opt_btime: Option<u64>,
    search_moves: Vec<LongAlgebricNotationMove>,
    moves: Vec<LongAlgebricNotationMove>,
    opt_position: Option<Position>,
}
impl Configuration {
    pub fn new() -> Self {
        Configuration {
            opt_depth: None,
            opt_time_per_move_in_ms: None,
            moves: vec![],
            opt_wtime: None,
            opt_btime: None,
            search_moves: vec![],
            opt_position: None,
        }
    }
}

pub enum UciResult {
    Quit,
    Continue,
    BestMove(LongAlgebricNotationMove),
}

#[derive(Debug)]
struct CommandError {
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
struct HandleEventError {
    event: Event,
    error: String,
}
impl HandleEventError {
    pub fn new(event: Event, error: String) -> Self {
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

fn parse_go(go_command: String) -> Result<Command, CommandError> {
    let mut result = true;
    let go_vec = go_command.split_whitespace().collect::<Vec<&str>>();
    // TODO: use trait default
    let mut parsed = GoStruct {
        depth: None,
        movetime: None,
        infinite: false,
        wtime: None,
        btime: None,
        search_moves: vec![],
    };
    for i in 1..go_vec.len() {
        match go_vec[i] {
            "depth" => {
                parsed.depth = Some(go_vec[i + 1].parse().unwrap());
            }
            "movetime" => {
                parsed.movetime = Some(go_vec[i + 1].parse().unwrap());
            }
            "wtime" => {
                parsed.wtime = Some(go_vec[i + 1].parse().unwrap());
            }
            "btime" => {
                parsed.btime = Some(go_vec[i + 1].parse().unwrap());
            }
            "infinite" => {
                parsed.infinite = true;
            }
            "searchmoves" => {
                parsed.search_moves = go_vec[i + 1..]
                    .to_vec()
                    .iter()
                    .map(|s| s.to_string())
                    .collect(); // Extract all moves
                break; // Stop after capturing moves
            }
            _ => result = false,
        }
    }
    if result {
        Ok(Command::Go(parsed))
    } else {
        Err(CommandError::new(
            format!("go command error: {}", go_command).to_string(),
        ))
    }
}
fn parse_position(position_command: String) -> Result<Command, CommandError> {
    let position_vec = position_command.split_whitespace().collect::<Vec<&str>>();

    // TODO: use trait default
    let mut parsed = PositionStruct {
        startpos: false,
        fen: None,
        moves: vec![],
    };
    match position_vec.as_slice() {
        // Case for startpos without moves
        ["position", "startpos"] => {
            parsed.startpos = true;
        }

        // Case for startpos with moves
        ["position", "startpos", "moves", moves @ ..] => {
            parsed.startpos = true;
            parsed.moves = moves.iter().map(|&m| m.to_string()).collect();
        }

        // Case for fen without moves
        ["position", "fen", fen_part @ ..] if fen_part.len() == 6 => {
            parsed.fen = Some(fen_part.join(" "));
        }

        // Case for fen with moves
        ["position", "fen", fen_part @ ..] if fen_part.len() > 6 && fen_part[6] == "moves" => {
            let (fen, moves) = fen_part.split_at(6);
            parsed.fen = Some(fen.join(" "));
            parsed.moves = moves[1..].iter().map(|&m| m.to_string()).collect(); // Skip "moves"
        }

        _ => {
            println!("Error: Invalid position command");
        }
    }
    Err(CommandError::new(
        format!("position command error: {}", position_command).to_string(),
    ))
}

impl Configuration {
    pub fn read_input(stdin: &mut Stdin) -> Result<Command, CommandError> {
        let mut input = String::new();
        stdin
            .lock()
            .read_line(&mut input)
            .expect("Failed to read line");
        let input = input.trim();
        match input {
            "uci" => Ok(Command::Uci),
            "isready" => Ok(Command::IsReady),
            cmd if cmd.starts_with("position") => parse_position(cmd.to_string()),
            cmd if cmd.starts_with("go") => parse_go(cmd.to_string()),
            "stop" => Ok(Command::Stop),
            "quit" => Ok(Command::Quit),
            _ => Err(CommandError::new(
                format!("Invalid command input: {}", input).to_string(),
            )),
        }
    }
    fn handle_event(
        &mut self,
        event: &Event,
        stdout: &mut Stdout,
    ) -> Result<UciResult, HandleEventError> {
        let mut result = Ok(UciResult::Continue);
        match event {
            Event::Write(s) => {
                writeln!(stdout, "{}", s).unwrap();
            }
            Event::StartPos => {
                let position: fen::Position = fen::Position::build_initial_position();
                self.opt_position = Some(position);
            }
            Event::Fen(fen) => {
                let position = fen::FEN::decode(fen).expect("Failed to decode FEN");
                self.opt_position = Some(position);
            }
            // Go command
            Event::Depth(depth) => self.opt_depth = Some(*depth),
            Event::SearchInfinite => self.opt_time_per_move_in_ms = None,
            Event::TimePerMoveInMs(time) => {
                self.opt_time_per_move_in_ms = Some(*time);
            }
            event @ Event::Moves(moves) => match moves_validation(moves) {
                Ok(valid_moves) => self.moves = valid_moves,
                Err(err) => result = Err(HandleEventError::new(event.clone(), err)),
            },
            Event::Wtime(wtime) => self.opt_wtime = Some(*wtime),
            Event::Btime(btime) => self.opt_btime = Some(*btime),
            event @ Event::SearchMoves(search_moves) => match moves_validation(search_moves) {
                Ok(valid_moves) => self.search_moves = valid_moves,
                Err(err) => result = Err(HandleEventError::new(event.clone(), err)),
            },
            Event::Stop => {
                match self.opt_position {
                    None => {
                        result = Err(HandleEventError::new(
                            Event::Stop,
                            "No bestmove since no valid position has been entered.".to_string(),
                        ))
                    }
                    Some(_) => {
                        // TODO:
                        // - stop current search
                        // - get bestmove e2e4
                        let best_move =
                            LongAlgebricNotationMove::build_from_str("e2e4".to_string()).unwrap();
                        result = Ok(UciResult::BestMove(best_move));
                    }
                }
            }
            Event::Quit => result = Ok(UciResult::Quit),
        }
        result
    }
    fn handle_command(&mut self, command: Command) -> Vec<Event> {
        let mut events: Vec<Event> = vec![];
        match command {
            Command::Uci => events.extend(vec![
                Event::Write("id name RandomEngine".to_string()),
                Event::Write("id author Christophe Le Cam".to_string()),
                Event::Write("uciok".to_string()),
            ]),
            Command::IsReady => events.push(Event::Write("readyok".to_string())),
            Command::Position(pos) => {
                events.push(Event::Write("Position received".to_string()));
                if pos.startpos {
                    events.push(Event::Write("Set board to starting position.".to_string()));
                    events.push(Event::StartPos);
                } else if let Some(fen_str) = pos.fen {
                    events.push(Event::Write(
                        format!("Set board to FEN: {}", fen_str).to_string(),
                    ));
                    events.push(Event::Fen(fen_str));
                }
                if !pos.moves.is_empty() {
                    events.push(Event::Write(
                        format!("Moves played: {:?}", pos.moves).to_string(),
                    ));
                    events.push(Event::Moves(pos.moves));
                }
            }
            Command::Go(go) => {
                if let Some(d) = go.depth {
                    events.push(Event::Write(
                        format!("Searching to depth: {}", d).to_string(),
                    ));
                    events.push(Event::Depth(d));
                }
                if let Some(time) = go.movetime {
                    events.push(Event::Write(
                        format!("Time for move: {} ms", time).to_string(),
                    ));
                    events.push(Event::TimePerMoveInMs(time));
                }
                if go.infinite {
                    events.push(Event::Write("Searching indefinitely...".to_string()));
                    events.push(Event::SearchInfinite);
                }
                if let Some(wtime) = go.wtime {
                    events.push(Event::Write(
                        format!("White time left: {} ms", wtime).to_string(),
                    ));
                    events.push(Event::Wtime(wtime));
                }
                if let Some(btime) = go.btime {
                    events.push(Event::Write(
                        format!("Black time left: {} ms", btime).to_string(),
                    ));
                    events.push(Event::Btime(btime));
                }
                if !go.search_moves.is_empty() {
                    events.push(Event::Write(format!(
                        "Limit search to these moves: {:?}",
                        go.search_moves
                    )));
                    events.push(Event::SearchMoves(go.search_moves));
                }
            }
            Command::Stop => {
                events.push(Event::Write("Stopping search.".to_string()));
                events.push(Event::Stop);
            }
            Command::Quit => {
                events.push(Event::Write("Exiting engine".to_string()));
                events.push(Event::Quit);
            }
        }
        events
    }
}

fn moves_validation(moves: &Vec<String>) -> Result<Vec<LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match LongAlgebricNotationMove::build_from_str(m.clone()) {
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

fn best_move_action(
    stdout: &mut Stdout,
    best_move: LongAlgebricNotationMove,
) -> Result<(), io::Error> {
    writeln!(stdout, "{}", best_move.cast())
}
fn write_err(stdout: &mut Stdout, err: String) -> Result<(), io::Error> {
    writeln!(stdout, "{}", err)
}

pub fn uci_loop() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut configuration = Configuration::new();

    'main_loop: loop {
        let command = Configuration::read_input(&mut stdin).expect("Invalid command");
        let events = configuration.handle_command(command);
        for event in &events {
            let uci_result = configuration.handle_event(event, &mut stdout);
            match uci_result {
                Ok(UciResult::Continue) => {}
                Ok(UciResult::Quit) => break 'main_loop,
                Ok(UciResult::BestMove(best_move)) => _ = best_move_action(&mut stdout, best_move),
                Err(HandleEventError { event, error }) => {
                    _ = write_err(&mut stdout, format!("{:?}{}", event, error))
                }
            }
            stdout.flush().unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci_input() {
        let input = "position startpos moves e2e4 e7e5 g1f3";
        // TODO
    }
}