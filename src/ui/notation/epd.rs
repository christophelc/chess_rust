use std::error::Error;
use std::fmt;

use crate::entity::game::component::{bitboard::zobrist, game_state};

use super::{fen::{self, EncodeUserInput, Position}, long_notation, san::{self, san_to_long_notation}};

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
    Am {
        san: String,
        long_notation: String,
    },
    Bm {
        san: String,
        long_notation: String,
    },
    Id(String),
}
impl fmt::Display for EpdOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            EpdOperation::Am {san, long_notation: _ } => write!(f, "am {}", san),
            EpdOperation::Bm { san, long_notation: _ } => write!(f, "bm {}", san),
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
    fn san_to_long_notation(san_str: &str, lang: &san::Lang, game: &game_state::GameState, zobrist_table: &zobrist::Zobrist) -> Option<String> {
        let moves = game.gen_moves();
        san::san_to_long_notation(san_str, &moves, lang, game, zobrist_table)
    }
    pub fn parse_operation(operation: &str, lang: &san::Lang, game: &game_state::GameState, zobrist_table: &zobrist::Zobrist) -> Result<EpdOperation, EpdOperationError> {
        let mut parts = operation.splitn(2, ' '); // Split into key and value
        let key = parts
            .next()
            .ok_or_else(|| EpdOperationError::InvalidFormat(operation.to_string()))?;
        let value = parts
            .next()
            .ok_or_else(|| EpdOperationError::InvalidFormat(operation.to_string()))?;

        match key.to_lowercase().as_str() {
            "am" => {
                match Self::san_to_long_notation(value, lang, game, zobrist_table) {
                    Some(long_notation) => Ok(EpdOperation::Am { san: value.to_string(), long_notation: long_notation.to_string() }),
                    None => Err(EpdOperationError::InvalidMove(value.to_string()))
                }
            }
            "bm" => {
                match Self::san_to_long_notation(value, lang, game, zobrist_table) {
                    Some(long_notation) => Ok(EpdOperation::Bm { san: value.to_string(), long_notation: long_notation.to_string() }),
                    None => Err(EpdOperationError::InvalidMove(value.to_string()))
                }                
            }
            "id" => Ok(EpdOperation::Id(value.trim_matches('"').to_string())),
            _ => Err(EpdOperationError::UnknownOperation(operation.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct Epd {
    position: Position,
    operations: Vec<EpdOperation>,
}
impl Epd {
    fn new(position: Position, operations: Vec<EpdOperation>) -> Self {
        Self {
            position,
            operations,
        }
    }
    fn position(&self) -> &Position {
        &self.position
    }
    fn operations(&self) -> &Vec<EpdOperation> {
        &self.operations
    }
    pub fn parse_operations(
        raw_operations: Vec<String>,
        lang: &san::Lang, 
        game: &game_state::GameState, 
        zobrist_table: &zobrist::Zobrist        
    ) -> Result<Vec<EpdOperation>, Vec<EpdOperationError>> {
        let mut operations = Vec::new();
        let mut errors = Vec::new();

        for operation in raw_operations {
            match EpdOperation::parse_operation(&operation, lang, game, zobrist_table) {
                Ok(op) => operations.push(op),
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

impl Epd {
    fn decode(epd: &str, lang: &san::Lang) -> Result<Epd, EpdError> {
        // ensure we have the first 4 elements of a fen and at least one operation
        let parts: Vec<&str> = epd.split_whitespace().collect();
        if parts.is_empty() || parts.len() <= 4 {
            return Err(EpdError::InvalidFormat(epd.to_string()));
        }
        let fen_part = parts[0..4].join(" ");
        let fen_with_dummy_fields = format!("{fen_part} 0 1");
        // list the operations speared by ;
        let parts = parts[4..].join(" ");
        let parts: Vec<&str> = parts.split(";").collect();
        if parts.is_empty() {
            return Err(EpdError::InvalidFormat(epd.to_string()));
        }
        match fen::Fen::decode(&fen_with_dummy_fields) {
            Ok(position) => {
                let zobrist_table = zobrist::Zobrist::new();
                let game = game_state::GameState::new(position, &zobrist_table);
                let raw_operations = parts[0..]
                    .iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let operations_or_errors = Self::parse_operations(
                    raw_operations,
                    lang, 
                    &game, 
                    &zobrist_table                    
                );
                match operations_or_errors {
                    Ok(operations) => Ok(Epd::new(position, operations)),
                    Err(errs) => Err(EpdError::ParseError(errs)),
                }
            }
            Err(fen_error) => Err(EpdError::FenPartError(fen_error)),
        }
    }
    fn encode(epd: &Epd) -> Result<String, EpdError> {
        match fen::Fen::encode(epd.position()) {
            Ok(full_fen) => {
                let truncated_fen = full_fen
                    .split_whitespace()
                    .take(4) // Keep only the first four fields
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
    fn test_encode_decode_epd() {
        let epd_str = "1k1r4/pp1b1R2/3q2pp/4p3/2B5/4Q3/PPP2B2/2K5 b - - bm Qd1+;id \"BK.01\";";
        let result = Epd::decode(epd_str, &san::Lang::LangEn);
        println!("{:?}", result);
        assert!(result.is_ok());
        let epd = result.unwrap();
        assert_eq!(
            *epd.operations(),
            vec![
                EpdOperation::Bm { san: "Qd1+".to_string(), long_notation: "d6d1".to_string() },
                EpdOperation::Id("BK.01".to_string())
            ]
        );
        let epd_to_str = Epd::encode(&epd);
        assert!(epd_to_str.is_ok());
        assert_eq!(epd_to_str.unwrap(), epd_str);
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
