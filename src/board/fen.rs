use crate::board::square;
use crate::board::coord;

pub struct Position {
    squares: [square::Square; 64],
    castling_white_queen_side: bool,
    castling_white_king_side: bool,    
    castling_black_queen_side: bool,
    castling_black_king_side: bool,
    player_turn: square::Color,
    pawn_en_passant: Option<coord::Coord>,
    half_moves: u16, // 50 moves rule without capturing a piece or moving a pawn
    moves: u16,
}

impl Position {
    pub fn new() -> Position {
        let fen_str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        return FEN::decode(fen_str).expect("Failed to decode FEN");
    }
    pub fn chessboard(&self) -> ChessBoard {
        let mut bd = ChessBoard::empty();
        for (i, square) in self.squares.iter().enumerate() {
            let row = i / 8;
            let col = i % 8;
            bd.squares[row][col] = *square; // Copy the square from the Position to the ChessBoard
        }
        bd
    }
}

use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum FenError {
    InvalidInput(String),
    InvalidPiece(char),
    InvalidPosition,
    InvalidEnPassantCapture(String),
    ParseError(String),
}

impl Error for FenError {}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            FenError::InvalidInput(ref desc) => write!(f, "Invalid input: {}", desc),
            FenError::InvalidPiece(piece) => write!(f, "Invalid piece character in FEN: {}", piece),
            FenError::InvalidPosition => write!(f, "Invalid position data in FEN"),
            FenError::InvalidEnPassantCapture(ref str) => write!(f, "Invalid Capture en passant: {}", str),
            FenError::ParseError(ref desc) => write!(f, "Parse error: {}", desc),
        }
    }
}

pub trait EncodeUserInput {
    fn decode(input: &str) -> Result<Position, FenError>;
    fn encode(position: &Position) -> Result<String, FenError>;    
}

pub(crate) struct FEN;

impl EncodeUserInput for FEN {
    fn decode(fen: &str) -> Result<Position, FenError> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(FenError::InvalidInput("FEN string must have six parts".to_string()));
        }

        let mut squares = [square::Square::Empty; 64];
        let mut row = 0;
        let mut col = 0;

        for c in parts[0].chars() {
            match c {
                '1'..='8' => {
                    let empty_spaces = c.to_digit(10).unwrap() as usize;
                    col += empty_spaces;
                },
                '/' => {
                    if col != 8 {
                        return Err(FenError::InvalidPosition);
                    }
                    row += 1;
                    col = 0;
                },
                _ => {
                    if let Some(square) = char_to_square(c) {
                        squares[row * 8 + col] = square;
                        col += 1;
                    } else {
                        return Err(FenError::InvalidPiece(c));
                    }
                }
            }
            if col > 8 { return Err(FenError::InvalidPosition); }
        }

        if row != 7 || col != 8 {
            return Err(FenError::InvalidPosition);
        }

        let player_turn = match parts[1] {
            "w" => square::Color::White,
            "b" => square::Color::Black,
            _ => return Err(FenError::ParseError("Invalid player turn symbol".to_string())),
        };

        let pawn_en_passant = if parts[3] != "-" {
            let col = parts[3].chars().next().unwrap();
            let row = parts[3][1..2].parse::<u8>().map_err(|_| FenError::ParseError("Invalid en passant square".to_string()))?;
            match Coord::from(col, row) {
                Ok(coord) => Some(coord),
                Err(_) => return Err(FenError::InvalidEnPassantCapture(format!("{}{}", col, row))),
            }
        } else {
            None
        };

        let half_moves = parts[4].parse::<u16>()
            .map_err(|_| FenError::ParseError("Failed to parse half-moves".to_string()))?;
        let moves = parts[5].parse::<u16>()
            .map_err(|_| FenError::ParseError("Failed to parse full move number".to_string()))?;

        Ok(Position {
            squares,
            castling_white_king_side: parts[2].contains('K'),
            castling_white_queen_side: parts[2].contains('Q'),
            castling_black_king_side: parts[2].contains('k'),
            castling_black_queen_side: parts[2].contains('q'),
            player_turn,
            pawn_en_passant,
            half_moves,
            moves,
        })
    }

    fn encode(position: &Position) -> Result<String, FenError> {
        let mut result = String::new();
        let mut empty_count = 0;

        for row in 0..8 {
            if row > 0 {
                if empty_count > 0 {
                    result.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                result.push('/');
            }

            for col in 0..8 {
                let index = row * 8 + col;
                match position.squares[index] {
                    square::Square::Empty => empty_count += 1,
                    square::Square::NonEmpty { piece, color } => {
                        if empty_count > 0 {
                            result.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        result.push(square_to_char(piece, color));
                    }
                }
            }

            if empty_count > 0 {
                result.push_str(&empty_count.to_string());
                empty_count = 0;
            }
        }

        result.push(' ');

        result.push_str(match position.player_turn {
            square::Color::White => "w",
            square::Color::Black => "b",
        });

        result.push(' ');

        let mut castling = String::new();
        if position.castling_white_king_side { castling.push('K'); }
        if position.castling_white_queen_side { castling.push('Q'); }        
        if position.castling_black_king_side { castling.push('k'); }
        if position.castling_black_queen_side { castling.push('q'); }        
        if castling.is_empty() { castling.push('-'); }
        result.push_str(&castling);

        result.push(' ');

        match position.pawn_en_passant {
            Some(coord) => result.push_str(&format!("{}{}", coord.col, coord.row)),
            None => result.push('-'),
        }

        result.push(' ');
        result.push_str(&position.half_moves.to_string());
        result.push(' ');
        result.push_str(&position.moves.to_string());

        Ok(result)
    }
}

// Helper function for converting a Square to a FEN character
fn square_to_char(piece: square::Piece, color: square::Color) -> char {
    match (piece, color) {
        (square::Piece::Rook, square::Color::White) => 'R',
        (square::Piece::Knight, square::Color::White) => 'N',
        (square::Piece::Bishop, square::Color::White) => 'B',
        (square::Piece::Queen, square::Color::White) => 'Q',
        (square::Piece::King, square::Color::White) => 'K',
        (square::Piece::Pawn, square::Color::White) => 'P',
        (square::Piece::Rook, square::Color::Black) => 'r',
        (square::Piece::Knight, square::Color::Black) => 'n',
        (square::Piece::Bishop, square::Color::Black) => 'b',
        (square::Piece::Queen, square::Color::Black) => 'q',
        (square::Piece::King, square::Color::Black) => 'k',
        (square::Piece::Pawn, square::Color::Black) => 'p',
    }
}

// Helper function for converting a character to a Square
fn char_to_square(c: char) -> Option<square::Square> {
    let black = square::Color::Black;
    let white = square::Color::White;
    match c {
        'r' => Some(square::Square::build_piece(square::Piece::Rook, black)),
        'n' => Some(square::Square::build_piece(square::Piece::Knight, black)),
        'b' => Some(square::Square::build_piece(square::Piece::Bishop, black)),
        'q' => Some(square::Square::build_piece(square::Piece::Queen, black)),
        'k' => Some(square::Square::build_piece(square::Piece::King, black)),
        'p' => Some(square::Square::build_piece(square::Piece::Pawn, black)),
        'R' => Some(square::Square::build_piece(square::Piece::Rook, white)),
        'N' => Some(square::Square::build_piece(square::Piece::Knight, white)),
        'B' => Some(square::Square::build_piece(square::Piece::Bishop, white)),
        'Q' => Some(square::Square::build_piece(square::Piece::Queen, white)),
        'K' => Some(square::Square::build_piece(square::Piece::King, white)),
        'P' => Some(square::Square::build_piece(square::Piece::Pawn, white)),
        '1'..='8' => None, // Empty squares will be handled by the caller (decode function)
        _ => None, // Invalid character
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_starting_position() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let position = FEN::decode(fen).expect("Failed to decode FEN");

        // Check if the pieces are in the correct initial positions
        assert_eq!(position.squares[0], square::Square::build_piece(Piece::Rook, Color::Black));
        assert_eq!(position.squares[7], square::Square::build_piece(Piece::Rook, Color::Black));
        assert_eq!(position.squares[56], square::Square::build_piece(Piece::Rook, Color::White));
        assert_eq!(position.squares[63], square::Square::build_piece(Piece::Rook, Color::White));

        // Check player turn
        assert_eq!(position.player_turn, square::Color::White);

        // Check castling rights
        assert!(position.castling_white_king_side);
        assert!(position.castling_white_queen_side);        
        assert!(position.castling_black_king_side);
        assert!(position.castling_black_queen_side);        

        // Check en passant
        assert_eq!(position.pawn_en_passant, None);

        // Check move counters
        assert_eq!(position.half_moves, 0);
        assert_eq!(position.moves, 1);
    }

    use square::Piece;
    use square::Color;
    use square::Square;
    
    #[test]
    fn test_encode_starting_position() {
        let position = Position {
            squares: [
                Square::build_piece(Piece::Rook, Color::Black), Square::build_piece(Piece::Knight, Color::Black), 
                Square::build_piece(Piece::Bishop, Color::Black), Square::build_piece(Piece::Queen, Color::Black),
                Square::build_piece(Piece::King, Color::Black), Square::build_piece(Piece::Bishop, Color::Black),
                Square::build_piece(Piece::Knight, Color::Black), Square::build_piece(Piece::Rook, Color::Black),
                Square::build_piece(Piece::Pawn, Color::Black), Square::build_piece(Piece::Pawn, Color::Black),
                Square::build_piece(Piece::Pawn, Color::Black), Square::build_piece(Piece::Pawn, Color::Black),
                Square::build_piece(Piece::Pawn, Color::Black), Square::build_piece(Piece::Pawn, Color::Black),
                Square::build_piece(Piece::Pawn, Color::Black), Square::build_piece(Piece::Pawn, Color::Black),
                Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty,
                Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty,
                Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty,
                Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty, Square::Empty,
                Square::build_piece(Piece::Pawn, Color::White), Square::build_piece(Piece::Pawn, Color::White),
                Square::build_piece(Piece::Pawn, Color::White), Square::build_piece(Piece::Pawn, Color::White),
                Square::build_piece(Piece::Pawn, Color::White), Square::build_piece(Piece::Pawn, Color::White),
                Square::build_piece(Piece::Pawn, Color::White), Square::build_piece(Piece::Pawn, Color::White),
                Square::build_piece(Piece::Rook, Color::White), Square::build_piece(Piece::Knight, Color::White), 
                Square::build_piece(Piece::Bishop, Color::White), Square::build_piece(Piece::Queen, Color::White),
                Square::build_piece(Piece::King, Color::White), Square::build_piece(Piece::Bishop, Color::White),
                Square::build_piece(Piece::Knight, Color::White), Square::build_piece(Piece::Rook, Color::White),
            ],
            castling_white_king_side: true,
            castling_white_queen_side: true,            
            castling_black_king_side: true,
            castling_black_queen_side: true,            
            player_turn: Color::White,
            pawn_en_passant: None,
            half_moves: 0,
            moves: 1,
        };

        let fen = FEN::encode(&position).expect("Failed to encode position");
        assert_eq!(fen, "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    }

    #[test]
    fn test_decode_encode_symmetry() {
        let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
        let position = FEN::decode(fen).expect("Failed to decode FEN");
        let encoded_fen = FEN::encode(&position).expect("Failed to encode position");

        assert_eq!(fen, encoded_fen);
    }

    #[test]
    fn test_decode_invalid_fen() {
        let invalid_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPP/RNBQKBNR w KQkq - 0 1"; // Missing a pawn in the last row
        let result = FEN::decode(invalid_fen);

        assert!(result.is_err());
    }

    #[test]
    fn test_encode_empty_position() {
        let empty_position = Position {
            squares: [Square::Empty; 64],
            castling_white_king_side: false,
            castling_white_queen_side: false,            
            castling_black_king_side: false,
            castling_black_queen_side: false,            
            player_turn: Color::White,
            pawn_en_passant: None,
            half_moves: 0,
            moves: 1,
        };

        let fen = FEN::encode(&empty_position).expect("Failed to encode position");
        assert_eq!(fen, "8/8/8/8/8/8/8/8 w - - 0 1");
    }
}

use coord::Coord;
use super::ChessBoard;

#[test]
fn test_decode_en_passant() {
    let fen = "8/8/8/8/4p3/8/3P4/8 w - e6 0 1"; // White pawn at d2 moved to d4, black pawn on e5 can capture en passant
    let position = FEN::decode(fen).expect("Failed to decode FEN");

    // Check if en passant is set correctly
    let expected_coord = Coord::from('e', 6).expect("Failed to create Coord");
    assert_eq!(position.pawn_en_passant, Some(expected_coord));
}
  
#[test]
fn test_encode_en_passant() {
    let position = Position {
        squares: [
            square::Square::Empty; 64 // Simplified: setting all squares to empty for this test
        ],
        castling_white_king_side: false,
        castling_white_queen_side: false,        
        castling_black_king_side: false,
        castling_black_queen_side: false,        
        player_turn: square::Color::White,
        pawn_en_passant: Some(Coord::from('e', 6).expect("Failed to create Coord")),
        half_moves: 0,
        moves: 1,
    };

    let fen = FEN::encode(&position).expect("Failed to encode position");
    assert!(fen.contains("E6"), "Expected en passant square 'e6' not found in encoded FEN");
}
