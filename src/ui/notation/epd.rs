use std::error::Error;
use std::fmt;

use crate::entity::game::component::{bitboard::zobrist, game_state};

use super::{
    fen::{self, EncodeUserInput, Position},
    san,
};

#[derive(Debug, PartialEq)]
pub enum EpdError {
    InvalidFormat(String),              // missing ;
    ParseError(Vec<EpdOperationError>), // Invalid operation format
    FenPartError(fen::FenError),        // Underlying FEN parsing error
}
impl fmt::Display for EpdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EpdError::InvalidFormat(ref str) => write!(f, "Invalid Epd format: {}", str),
            EpdError::ParseError(op_errors) => {
                let op_errors_str: Vec<_> =
                    op_errors.into_iter().map(|err| err.to_string()).collect();
                write!(f, "Epd parse error: {}", op_errors_str.join(", "))
            }
            EpdError::FenPartError(fen_error) => write!(f, "Fen error: {}", fen_error),
        }
    }
}
impl Error for EpdError {}

#[derive(Debug, PartialEq)]
pub enum EpdOperation {
    Am { san: String, long_notation: String },
    Bm { san: String, long_notation: String },
    Id(String),
    Comment(String),
}
impl fmt::Display for EpdOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            EpdOperation::Am {
                san,
                long_notation: _,
            } => write!(f, "am {}", san),
            EpdOperation::Bm {
                san,
                long_notation: _,
            } => write!(f, "bm {}", san),
            EpdOperation::Comment(str) => write!(f, "c0 \"{}\"", str),
            EpdOperation::Id(str) => write!(f, "id \"{}\"", str),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EpdOperationError {
    InvalidFormat(String),    // Operation doesn't match expected format
    UnknownOperation(String), // Unsupported operation type
    InvalidMove(String),
}
impl fmt::Display for EpdOperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EpdOperationError::InvalidFormat(str) => {
                write!(f, "EpdOperationError::InvalidFormat {}", str)
            }
            EpdOperationError::UnknownOperation(str) => write!(f, "Unknown EpdOperation {}", str),
            EpdOperationError::InvalidMove(str) => write!(f, "Invalid move {}", str),
        }
    }
}
impl EpdOperation {
    /// we can have multiple moves in case Ne5 means Nce5 or Nge5
    fn san_to_long_notation_epd(
        san_str: &str,
        lang: &san::Lang,
        game: &game_state::GameState,
        zobrist_table: &zobrist::Zobrist,
    ) -> Vec<String> {
        let moves = game.gen_moves();
        san::san_to_long_notation_epd(san_str, &moves, lang, game, zobrist_table)
    }
    pub fn parse_operation(
        operation: &str,
        lang: &san::Lang,
        game: &game_state::GameState,
        zobrist_table: &zobrist::Zobrist,
    ) -> Result<Vec<EpdOperation>, EpdOperationError> {
        let mut parts = operation.splitn(2, ' '); // Split into key and value
        let key = parts
            .next()
            .ok_or_else(|| EpdOperationError::InvalidFormat(operation.to_string()))?;
        let value = parts
            .next()
            .ok_or_else(|| EpdOperationError::InvalidFormat(operation.to_string()))?;

        match key.to_lowercase().as_str() {
            // FIXME: manage multiple moves for one SAN notation
            "am" => {
                let long_notations =
                    Self::san_to_long_notation_epd(value, lang, game, zobrist_table);
                if long_notations.is_empty() {
                    return Err(EpdOperationError::InvalidMove(value.to_string()));
                }
                let operations: Vec<EpdOperation> = long_notations
                    .into_iter()
                    .map(|long_notation| EpdOperation::Am {
                        san: value.to_string(),
                        long_notation: long_notation.to_string(),
                    })
                    .collect();
                Ok(operations)
            }
            // FIXME: manage multiple moves for one SAN notation
            "bm" => {
                let long_notations =
                    Self::san_to_long_notation_epd(value, lang, game, zobrist_table);
                if long_notations.is_empty() {
                    return Err(EpdOperationError::InvalidMove(value.to_string()));
                }
                let operations: Vec<EpdOperation> = long_notations
                    .into_iter()
                    .map(|long_notation| EpdOperation::Bm {
                        san: value.to_string(),
                        long_notation: long_notation.to_string(),
                    })
                    .collect();
                Ok(operations)
            }
            "c0" => Ok(vec![EpdOperation::Comment(
                value.trim_matches('"').to_string(),
            )]),
            "id" => Ok(vec![EpdOperation::Id(value.trim_matches('"').to_string())]),
            _ => Err(EpdOperationError::UnknownOperation(operation.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct Epd {
    position: Position,
    is_full_fen: bool,
    operations: Vec<EpdOperation>,
}
impl Epd {
    fn new(position: Position, operations: Vec<EpdOperation>, is_full_fen: bool) -> Self {
        Self {
            position,
            is_full_fen,
            operations,
        }
    }
    pub fn position(&self) -> &Position {
        &self.position
    }
    pub fn operations(&self) -> &Vec<EpdOperation> {
        &self.operations
    }
    pub fn parse_operations(
        raw_operations: Vec<String>,
        lang: &san::Lang,
        game: &game_state::GameState,
        zobrist_table: &zobrist::Zobrist,
    ) -> Result<Vec<EpdOperation>, Vec<EpdOperationError>> {
        let mut operations = Vec::new();
        let mut errors = Vec::new();

        for operation in raw_operations {
            match EpdOperation::parse_operation(&operation, lang, game, zobrist_table) {
                Ok(ops) => operations.extend(ops),
                Err(err) => errors.push(err),
            }
        }

        if errors.is_empty() {
            Ok(operations) // Return all operations if no errors
        } else {
            Err(errors) // Return all errors if any occurred
        }
    }
}

impl fmt::Display for Epd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let result = Epd::encode(self).unwrap();
        write!(f, "{}", result)
        // let fen = fen::Fen::encode(&self.position).unwrap();
        // let operations_str: Vec<String> = self.operations.iter().map(|op| op.to_string()).collect();
        // write!(f, "{} {}", fen, operations_str.join(";"))
    }
}

impl Epd {
    pub fn decode(epd: &str, lang: &san::Lang) -> Result<Epd, EpdError> {
        let parts_operations: Vec<&str> = epd.split(';').collect();
        if parts_operations.len() <= 1 {
            return Err(EpdError::InvalidFormat(epd.to_string()));
        }
        // ensure we have the first 4 elements of a fen and at least one operation
        let parts: Vec<&str> = parts_operations[0].split_whitespace().collect();
        if parts.len() <= 4 {
            return Err(EpdError::InvalidFormat(epd.to_string()));
        }
        // Detect if we have a full FEN (6 fields) or a minimal FEN (4 fields)
        let fen_part;
        let operations_start_index;
        let is_full_fen;

        if parts.len() >= 6 && parts[4] == "-" {
            // Minimal FEN
            fen_part = format!("{} 0 1", parts[0..4].join(" "));
            is_full_fen = false;
            operations_start_index = 5;
        } else if parts.len() >= 6 && parts[5].parse::<u32>().is_ok() {
            // Full FEN (6 fields, ending with the full-move number)
            fen_part = parts[0..6].join(" ");
            is_full_fen = true;
            operations_start_index = 6;
        } else {
            // Minimal FEN (4 fields, add dummy half-move clock and full-move number)
            fen_part = format!("{} 0 1", parts[0..4].join(" "));
            is_full_fen = false;
            operations_start_index = 4;
        }
        // Extract operations after the FEN part
        let parts: Vec<&str> = epd.split_whitespace().collect();
        let operations_str = parts[operations_start_index..].join(" ");
        let operations: Vec<&str> = operations_str.split(';').collect();

        // Ensure we have at least one operation
        if operations.is_empty() || operations.iter().all(|s| s.trim().is_empty()) {
            return Err(EpdError::InvalidFormat(epd.to_string()));
        }
        match fen::Fen::decode(&fen_part) {
            Ok(position) => {
                let zobrist_table = zobrist::Zobrist::new();
                let game = game_state::GameState::new(position, &zobrist_table);
                let raw_operations = operations
                    .iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let operations_or_errors =
                    Self::parse_operations(raw_operations, lang, &game, &zobrist_table);
                match operations_or_errors {
                    Ok(operations) => Ok(Epd::new(position, operations, is_full_fen)),
                    Err(errs) => Err(EpdError::ParseError(errs)),
                }
            }
            Err(fen_error) => Err(EpdError::FenPartError(fen_error)),
        }
    }
    pub fn encode(epd: &Epd) -> Result<String, EpdError> {
        let n_parts = if epd.is_full_fen { 6 } else { 4 };
        match fen::Fen::encode(epd.position()) {
            Ok(minimal_or_full_fen) => {
                let truncated_fen = minimal_or_full_fen
                    .split_whitespace()
                    .take(n_parts)
                    .collect::<Vec<&str>>()
                    .join(" ");
                let operations: Vec<_> = epd
                    .operations()
                    .into_iter()
                    .map(|op| op.to_string())
                    .collect();
                Ok(format!("{} {};", truncated_fen, operations.join(";")))
            }
            Err(fen_error) => Err(EpdError::FenPartError(fen_error)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_decode_epd_mininal_fen() {
        let epd_str = "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - bm Qd1+;id \"BK.01\";";
        let result = Epd::decode(epd_str, &san::Lang::LangEn);
        assert!(result.is_ok());
        let epd = result.unwrap();
        assert_eq!(
            *epd.operations(),
            vec![
                EpdOperation::Bm {
                    san: "Qd1+".to_string(),
                    long_notation: "d6d1".to_string()
                },
                EpdOperation::Id("BK.01".to_string())
            ]
        );
        let epd_to_str = Epd::encode(&epd);
        assert!(epd_to_str.is_ok());
        assert_eq!(epd_to_str.unwrap(), epd_str);
    }
    #[test]
    fn test_encode_decode_epd_full_fen() {
        let epd_str = "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - 1 5 bm Qd1+;id \"BK.01\";";
        let result = Epd::decode(epd_str, &san::Lang::LangEn);
        assert!(result.is_ok());
        let epd = result.unwrap();
        assert_eq!(
            *epd.operations(),
            vec![
                EpdOperation::Bm {
                    san: "Qd1+".to_string(),
                    long_notation: "d6d1".to_string()
                },
                EpdOperation::Id("BK.01".to_string())
            ]
        );
        let epd_to_str = Epd::encode(&epd);
        assert!(epd_to_str.is_ok());
        assert_eq!(epd_to_str.unwrap(), epd_str);
    }
    #[test]
    fn test_ambiguous_move() {
        let epd_str = "r1bqk2r/ppp2ppp/2n5/4P3/2Bp2n1/5N1P/PP1N1PP1/R2Q1RK1 b kq - 1 10 id \"CCR03\"; bm Nh6; am Ne5;";
        let epd = Epd::decode(epd_str, &san::Lang::LangEn).unwrap();
        let expected = EpdOperation::Am {
            san: "Ne5".to_string(),
            long_notation: "g4e5".to_string(),
        };
        let expected2 = EpdOperation::Am {
            san: "Ne5".to_string(),
            long_notation: "c6e5".to_string(),
        };
        assert!(epd.operations.contains(&expected));
        assert!(epd.operations.contains(&expected2))
    }
    #[test]
    fn test_encode_operation_error() {
        let epd_str = "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - bx Qd1+;id \"BK.01\";";
        let result = Epd::decode(epd_str, &san::Lang::LangFr);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            EpdError::ParseError(vec!(EpdOperationError::UnknownOperation(
                "bx Qd1+".to_string()
            )))
        )
    }
}
