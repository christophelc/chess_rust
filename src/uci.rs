use std::error::Error;
use std::fmt::Display;
use std::io::{self, stdout, BufRead, Stdin, Stdout, Write};
use std::ops::Index;

use crate::board::bitboard::BitBoardMove;
use crate::board::fen::{self, EncodeUserInput, Position};
use crate::board::square::TypePiece;
use crate::board::{bitboard, square, ChessBoard};

#[derive(Debug)]
pub enum Command {
    Uci,     // "uci" command, no additional data needed
    IsReady, // "isready" command, no additional data needed
    Position(PositionStruct),
    Go(GoStruct),
    Stop, // "stop" command to stop search
    Quit, // "quit" command to exit the engine
}

#[derive(Debug)]
struct PositionStruct {
    // "position" command, with optional FEN and moves
    startpos: bool,      // `true` if the starting position is requested
    fen: Option<String>, // The FEN string, if specified (None if using startpos)
    moves: Vec<String>,  // A list of moves played after the position
}

#[derive(Debug)]
struct GoStruct {
    // "go" command, with search parameters
    depth: Option<u32>,        // Optional depth to search
    movetime: Option<u32>,     // Optional maximum time for the move (in ms)
    infinite: bool,            // If true, search indefinitely until told to stop
    wtime: Option<u64>,        // White time left,
    btime: Option<u64>,        // Black time left
    search_moves: Vec<String>, // Restrict search to this moves only
}
fn promotion2type_piece(opt_promotion_as_char: Option<char>) -> Result<Option<TypePiece>, String> {
    match opt_promotion_as_char {
        None => Ok(None),
        Some('q') => Ok(Some(TypePiece::Queen)),
        Some('r') => Ok(Some(TypePiece::Rook)),
        Some('n') => Ok(Some(TypePiece::Knight)),
        Some('b') => Ok(Some(TypePiece::Bishop)),
        Some(p) => Err(format!(
            "Unknow promotion piece: '{}'. Valid pieces are: q, r, n",
            p
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LongAlgebricNotationMove {
    start: u8,
    end: u8,
    opt_promotion: Option<TypePiece>,
}
impl LongAlgebricNotationMove {
    pub fn new(start: u8, end: u8, opt_promotion: Option<TypePiece>) -> Self {
        LongAlgebricNotationMove {
            start,
            end,
            opt_promotion,
        }
    }
    pub fn build_from_str(move_str: &str) -> Result<Self, String> {
        let mut result = Err(format!("Invalid move: {}", move_str));
        if move_str.len() >= 4 && move_str.len() <= 5 {
            let from_square = &move_str[0..2]; // First two characters (e.g., "e2")
            let to_square = &move_str[2..4]; // Last two characters (e.g., "e4")
            let from_index = square_to_index(from_square);
            let to_index = square_to_index(to_square);
            let opt_promotion = promotion2type_piece(move_str.chars().nth(4))?;
            if from_index < 64 && to_index < 64 {
                result = Ok(LongAlgebricNotationMove {
                    start: from_index,
                    end: to_index,
                    opt_promotion,
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
    pub fn start(&self) -> u8 {
        self.start
    }
    pub fn end(&self) -> u8 {
        self.end
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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parameters {
    opt_depth: Option<u32>,
    opt_time_per_move_in_ms: Option<u32>,
    opt_wtime: Option<u64>,
    opt_btime: Option<u64>,
    search_moves: Vec<LongAlgebricNotationMove>,
}

#[derive(Clone)]
pub struct Configuration {
    parameters: Parameters,
    opt_position: Option<Position>,
}
impl Configuration {
    pub fn new() -> Self {
        Configuration {
            parameters: Parameters::default(),
            opt_position: None,
        }
    }
    fn execute_command(&mut self, command: Command, stdout: &mut Stdout) -> bool {
        let mut is_quit = false;
        let events = self.handle_command(command);
        for event in &events {
            // pdate the configuration
            let uci_result = self.handle_event(event, stdout);
            // quit, stop and show best move or continue
            match uci_result {
                Ok(UciResult::Continue) => {}
                Ok(UciResult::Quit) => {
                    is_quit = true;
                }
                Ok(UciResult::BestMove(best_move)) => _ = best_move_action(stdout, best_move),
                Err(HandleEventError { event, error }) => {
                    _ = write_err(stdout, format!("{:?}{}", event, error))
                }
            }
        }
        is_quit
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
    for i in (1..go_vec.len()).step_by(2) {
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
            Ok(Command::Position(parsed))
        }

        // Case for startpos with moves
        ["position", "startpos", "moves", moves @ ..] => {
            parsed.startpos = true;
            parsed.moves = moves.iter().map(|&m| m.to_string()).collect();
            Ok(Command::Position(parsed))
        }

        // Case for fen without moves
        ["position", "fen", fen_part @ ..] if fen_part.len() == 6 => {
            parsed.fen = Some(fen_part.join(" "));
            Ok(Command::Position(parsed))
        }

        // Case for fen with moves
        ["position", "fen", fen_part @ ..] if fen_part.len() > 6 && fen_part[6] == "moves" => {
            let (fen, moves) = fen_part.split_at(6);
            parsed.fen = Some(fen.join(" "));
            parsed.moves = moves[1..].iter().map(|&m| m.to_string()).collect(); // Skip "moves";
            Ok(Command::Position(parsed))
        }

        _ => Err(CommandError::new(
            format!("position command error: {}", position_command).to_string(),
        )),
    }
}

impl Configuration {
    pub fn parse_input(input: &str) -> Result<Command, CommandError> {
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
                stdout.flush().unwrap();
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
            Event::Depth(depth) => self.parameters.opt_depth = Some(*depth),
            Event::SearchInfinite => self.parameters.opt_time_per_move_in_ms = None,
            Event::TimePerMoveInMs(time) => {
                self.parameters.opt_time_per_move_in_ms = Some(*time);
            }
            event @ Event::Moves(moves) => match moves_validation(moves) {
                Ok(valid_moves) => {
                    if let Some(position) = self.opt_position {
                        let mut bit_position = bitboard::BitPosition::from(position);
                        for m in valid_moves {
                            let color = bit_position.bit_position_status().player_turn();
                            match check_move(color, m, &bit_position.bit_boards_white_and_black()) {
                                Err(err) => {
                                    result =
                                        Err(HandleEventError::new(event.clone(), err.to_string()));
                                    break;
                                }
                                Ok(b_move) => {
                                    bit_position = bit_position.move_piece(&b_move);
                                    self.opt_position = Some(bit_position.to());
                                }
                            }
                        }
                    } else {
                        result = Err(HandleEventError::new(
                            event.clone(),
                            "no configuration defined".to_string(),
                        ));
                    }
                }
                Err(err) => result = Err(HandleEventError::new(event.clone(), err)),
            },
            Event::Wtime(wtime) => self.parameters.opt_wtime = Some(*wtime),
            Event::Btime(btime) => self.parameters.opt_btime = Some(*btime),
            event @ Event::SearchMoves(search_moves) => match moves_validation(search_moves) {
                Ok(valid_moves) => self.parameters.search_moves = valid_moves,
                Err(err) => result = Err(HandleEventError::new(event.clone(), err.to_string())),
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
                        let best_move = LongAlgebricNotationMove::build_from_str("e2e4").unwrap();
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

// The start square must contain a piece
fn check_move(
    player_turn: square::Color,
    m: LongAlgebricNotationMove,
    bitboard_white_and_black: &bitboard::BitBoardsWhiteAndBlack,
) -> Result<BitBoardMove, String> {
    let start_square = bitboard_white_and_black.peek(m.start());
    let end_square = bitboard_white_and_black.peek(m.end());
    match (start_square, end_square) {
        (square::Square::Empty, _) => Err(format!("empty start square {}", m.start())),
        (square::Square::NonEmpty(piece), square::Square::Empty) => Ok(BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion)),
        (square::Square::NonEmpty(piece), square::Square::NonEmpty(capture)) if capture.color() != piece.color() => Ok(BitBoardMove::new(player_turn, piece.type_piece(), m.start(), m.end(), None, m.opt_promotion)),
        (square::Square::NonEmpty(_), square::Square::NonEmpty(_)) => Err(format!("Invalid move from {} to {} since the destination square contains a piece of the same color as the piece played." , m.start(), m.end())),
    }
}

fn moves_validation(moves: &Vec<String>) -> Result<Vec<LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match LongAlgebricNotationMove::build_from_str(&m) {
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
    let res = writeln!(stdout, "{}", best_move.cast());
    stdout.flush().unwrap();
    res
}
fn write_err(stdout: &mut Stdout, err: String) -> Result<(), io::Error> {
    let res = writeln!(stdout, "{}", err);
    stdout.flush().unwrap();
    res
}

// let mut uci_read = UciReadWrapper::new(&mut stdin);
// let mut configuration = Configuration::new();
pub fn uci_loop<T: UciRead>(mut uci_reader: T, configuration: &mut Configuration) {
    let mut stdout = io::stdout();

    let mut parameters_before = Configuration::new().parameters;

    loop {
        let input = uci_reader.uci_read();
        let command = Configuration::parse_input(&input).expect("Invalid command");
        if configuration.execute_command(command, &mut stdout) {
            break;
        }
        // check for configuration change
        if configuration.parameters != parameters_before {
            println!("The parameters have changed");
        }
        parameters_before = configuration.parameters.clone();
    }
}

pub trait UciRead {
    fn uci_read(&mut self) -> String;
}
struct UciReadWrapper<'a> {
    stdin: &'a mut Stdin,
}
impl<'a> UciReadWrapper<'a> {
    pub fn new(stdin: &'a mut Stdin) -> Self {
        UciReadWrapper { stdin }
    }
}

impl<'a> UciRead for UciReadWrapper<'a> {
    fn uci_read(&mut self) -> String {
        let mut input = String::new();
        self.stdin
            .lock()
            .read_line(&mut input)
            .expect("Failed to read line");
        input.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct UciReadTestWrapper<'a> {
        idx: usize,
        inputs: &'a [&'a str],
    }
    impl<'a> UciReadTestWrapper<'a> {
        pub fn new(inputs: &'a [&str]) -> Self {
            UciReadTestWrapper { idx: 0, inputs }
        }
    }
    impl<'a> UciRead for UciReadTestWrapper<'a> {
        fn uci_read(&mut self) -> String {
            if self.idx < self.inputs.len() {
                let result = self.inputs[self.idx];
                self.idx += 1;
                result.to_string()
            } else {
                "quit".to_string()
            }
        }
    }

    #[test]
    fn test_uci_input_start_pos() {
        let mut configuration = Configuration::new();
        let mut stdout = io::stdout();
        let input = "position startpos";
        let command = Configuration::parse_input(&input).expect("Invalid command");
        let is_quit = configuration.execute_command(command, &mut stdout);
        assert!(!is_quit);
        let fen = fen::FEN::encode(&configuration.opt_position.unwrap())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }
    #[test]
    fn test_uci_input_start_pos_with_moves() {
        let mut configuration = Configuration::new();
        let mut stdout = io::stdout();
        let input = "position startpos moves e2e4 e7e5 g1f3";
        let command = Configuration::parse_input(&input).expect("Invalid command");
        let is_quit = configuration.execute_command(command, &mut stdout);
        assert!(!is_quit);
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::FEN::encode(&configuration.opt_position.unwrap())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }
    #[test]
    fn test_uci_input_fen_pos() {
        let mut configuration = Configuration::new();
        let mut stdout = io::stdout();
        let input = format!("position fen {}", fen::FEN_START_POSITION);
        let command = Configuration::parse_input(&input).expect("Invalid command");
        let is_quit = configuration.execute_command(command, &mut stdout);
        assert!(!is_quit);
        let fen = fen::FEN::encode(&configuration.opt_position.unwrap())
            .expect("Failed to encode position");
        assert_eq!(fen, fen::FEN_START_POSITION);
    }
    #[test]
    fn test_uci_input_fen_pos_with_moves() {
        let mut configuration = Configuration::new();
        let mut stdout = io::stdout();
        let input = format!(
            "position fen {} moves e2e4 e7e5 g1f3",
            fen::FEN_START_POSITION
        );
        let command = Configuration::parse_input(&input).expect("Invalid command");
        let is_quit = configuration.execute_command(command, &mut stdout);
        assert!(!is_quit);
        let fen_str = "rnbqkbnr/pppp1ppp/8/4p3/4P3/5N2/PPPP1PPP/RNBQKB1R b KQkq - 1 2";
        let fen = fen::FEN::encode(&configuration.opt_position.unwrap())
            .expect("Failed to encode position");
        assert_eq!(fen, fen_str);
    }
    #[test]
    fn test_uci_input_default_parameters() {
        let mut configuration = Configuration::new();
        let mut stdout = io::stdout();
        let input = "position startpos";
        let command = Configuration::parse_input(&input).expect("Invalid command");
        let is_quit = configuration.execute_command(command, &mut stdout);
        assert!(!is_quit);
        let parameters = configuration.parameters;
        let expected = Parameters::default();
        assert_eq!(parameters, expected)
    }
    #[test]
    fn test_uci_input_modified_parameters() {
        let mut configuration = Configuration::new();
        let inputs = vec![
            "position startpos",
            "go depth 3 movetime 5000 wtime 3600000 btime 3600001",
        ];
        let uci_reader = UciReadTestWrapper::new(inputs.as_slice());
        uci_loop(uci_reader, &mut configuration);
        let parameters = configuration.parameters;
        let expected = Parameters {
            opt_depth: Some(3),
            opt_time_per_move_in_ms: Some(5000),
            opt_wtime: Some(3600000),
            opt_btime: Some(3600001),
            search_moves: vec![],
        };
        assert_eq!(parameters, expected)
    }
}
