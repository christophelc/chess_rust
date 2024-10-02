use std::fmt;

use super::{Command, CommandError, GoStruct, PositionStruct};

pub struct InputParser<'a> {
    input: &'a str,
}
impl<'a> fmt::Display for InputParser<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write the inner `input` string into the provided formatter
        write!(f, "{}", self.input)
    }
}
impl<'a> InputParser<'a> {
    pub fn new(input: &'a str) -> Self {
        InputParser { input }
    }
    pub fn parse_input(&self) -> Result<Command, CommandError> {
        match self.input {
            "uci" => Ok(Command::Uci),
            "xboard" => {
                println!("xboard protocol not supported");
                Ok(Command::Ignore)
            }
            "isready" => Ok(Command::IsReady),
            cmd if cmd.starts_with("position") => parse_position(cmd.to_string()),
            cmd if cmd.starts_with("go") => parse_go(cmd.to_string()),
            "ucinewgame" => Ok(Command::NewGame),
            "stop" => Ok(Command::Stop),
            "quit" => Ok(Command::Quit),
            _ => {
                //Err(CommandError::new(format!("Invalid command input: {}", self).to_string()))
                println!("Unknown command {}", self.input);
                Ok(Command::Ignore)
            }
        }
    }
}

fn parse_position(position_command: String) -> Result<Command, CommandError> {
    let position_vec = position_command.split_whitespace().collect::<Vec<&str>>();

    let mut parsed = PositionStruct::default();
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

fn parse_go(go_command: String) -> Result<Command, CommandError> {
    let mut result = true;
    let go_vec = go_command.split_whitespace().collect::<Vec<&str>>();
    let mut parsed = GoStruct::default();
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
            "winc" => {
                parsed.wtime_inc = Some(go_vec[i + 1].parse().unwrap());
            }
            "binc" => {
                parsed.btime_inc = Some(go_vec[i + 1].parse().unwrap());
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
