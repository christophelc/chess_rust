use crate::entity::game::component::{coord, square};

pub const FEN_START_POSITION: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Debug, Clone, Copy)]
pub struct Position {
    chessboard: ChessBoard,
    status: PositionStatus,
}

#[derive(Debug, Clone, Copy)]
pub struct PositionStatus {
    castling_white_queen_side: bool,
    castling_white_king_side: bool,
    castling_black_queen_side: bool,
    castling_black_king_side: bool,
    player_turn: square::Color,
    pawn_en_passant: Option<coord::Coord>,
    n_half_moves: u16, // 50 moves rule without capturing a piece or moving a pawn
    n_moves: u16,
}
impl PositionStatus {
    pub fn new() -> Self {
        PositionStatus {
            castling_white_queen_side: false,
            castling_white_king_side: false,
            castling_black_queen_side: false,
            castling_black_king_side: false,
            player_turn: square::Color::White,
            pawn_en_passant: None,
            n_half_moves: 0,
            n_moves: 0,
        }
    }
    // getters
    ////////////////////////
    pub fn castling_white_queen_side(&self) -> bool {
        self.castling_white_queen_side
    }

    pub fn castling_white_king_side(&self) -> bool {
        self.castling_white_king_side
    }

    pub fn castling_black_queen_side(&self) -> bool {
        self.castling_black_queen_side
    }

    pub fn castling_black_king_side(&self) -> bool {
        self.castling_black_king_side
    }

    pub fn player_turn(&self) -> square::Color {
        self.player_turn
    }

    pub fn pawn_en_passant(&self) -> Option<coord::Coord> {
        self.pawn_en_passant
    }

    pub fn n_half_moves(&self) -> u16 {
        self.n_half_moves
    }

    pub fn n_moves(&self) -> u16 {
        self.n_moves
    }

    // setters
    ////////////////////////
    pub fn set_castling_white_queen_side(&mut self, value: bool) {
        self.castling_white_queen_side = value;
    }

    pub fn set_castling_white_king_side(&mut self, value: bool) {
        self.castling_white_king_side = value;
    }

    pub fn set_castling_black_queen_side(&mut self, value: bool) {
        self.castling_black_queen_side = value;
    }

    pub fn set_castling_black_king_side(&mut self, value: bool) {
        self.castling_black_king_side = value;
    }

    pub fn set_player_turn(&mut self, value: square::Color) {
        self.player_turn = value;
    }

    pub fn set_pawn_en_passant(&mut self, value: Option<coord::Coord>) {
        self.pawn_en_passant = value;
    }

    pub fn set_n_half_moves(&mut self, value: u16) {
        self.n_half_moves = value;
    }

    pub fn set_n_moves(&mut self, value: u16) {
        self.n_moves = value;
    }
}

impl Position {
    pub fn build(chessboard: ChessBoard, status: PositionStatus) -> Self {
        Position { chessboard, status }
    }

    pub fn build_initial_position() -> Self {
        let fen_str = FEN_START_POSITION;
        Fen::decode(fen_str).expect("Failed to decode FEN")
    }

    // getters
    ////////////////////////
    pub fn chessboard(&self) -> &ChessBoard {
        &self.chessboard
    }

    pub fn into_chessboard(self) -> ChessBoard {
        self.chessboard
    }

    pub fn status(&self) -> &PositionStatus {
        &self.status
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
            FenError::InvalidEnPassantCapture(ref str) => {
                write!(f, "Invalid Capture en passant: {}", str)
            }
            FenError::ParseError(ref desc) => write!(f, "Parse error: {}", desc),
        }
    }
}

pub trait EncodeUserInput {
    fn decode(input: &str) -> Result<Position, FenError>;
    fn encode(position: &Position) -> Result<String, FenError>;
}

pub(crate) struct Fen;

impl EncodeUserInput for Fen {
    fn decode(fen: &str) -> Result<Position, FenError> {
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(FenError::InvalidInput(
                "FEN string must have six parts".to_string(),
            ));
        }

        let mut squares = [[square::Square::Empty; 8]; 8];
        let mut row = 0;
        let mut col = 0;

        for c in parts[0].chars() {
            match c {
                '1'..='8' => {
                    let empty_spaces = c.to_digit(10).unwrap() as usize;
                    col += empty_spaces;
                }
                '/' => {
                    if col != 8 {
                        return Err(FenError::InvalidPosition);
                    }
                    row += 1;
                    col = 0;
                }
                _ => {
                    if let Some(square) = char_to_square(c) {
                        squares[7 - row][col] = square;
                        col += 1;
                    } else {
                        return Err(FenError::InvalidPiece(c));
                    }
                }
            }
            if col > 8 {
                return Err(FenError::InvalidPosition);
            }
        }

        if row != 7 || col != 8 {
            return Err(FenError::InvalidPosition);
        }

        let player_turn = match parts[1] {
            "w" => square::Color::White,
            "b" => square::Color::Black,
            _ => {
                return Err(FenError::ParseError(
                    "Invalid player turn symbol".to_string(),
                ))
            }
        };

        let pawn_en_passant = if parts[3] != "-" {
            let col = parts[3].chars().next().unwrap();
            let row = parts[3][1..2]
                .parse::<u8>()
                .map_err(|_| FenError::ParseError("Invalid en passant square".to_string()))?;
            match Coord::from(col, row) {
                Ok(coord) => Some(coord),
                Err(_) => return Err(FenError::InvalidEnPassantCapture(format!("{}{}", col, row))),
            }
        } else {
            None
        };

        let n_half_moves = parts[4]
            .parse::<u16>()
            .map_err(|_| FenError::ParseError("Failed to parse half-moves".to_string()))?;
        let n_moves = parts[5]
            .parse::<u16>()
            .map_err(|_| FenError::ParseError("Failed to parse full move number".to_string()))?;

        Ok(Position {
            chessboard: ChessBoard::build(squares),
            status: PositionStatus {
                castling_white_king_side: parts[2].contains('K'),
                castling_white_queen_side: parts[2].contains('Q'),
                castling_black_king_side: parts[2].contains('k'),
                castling_black_queen_side: parts[2].contains('q'),
                player_turn,
                pawn_en_passant,
                n_half_moves,
                n_moves,
            },
        })
    }

    fn encode(position: &Position) -> Result<String, FenError> {
        let mut result = String::new();
        let mut empty_count = 0;

        for row in (0..8).rev() {
            if row < 7 {
                if empty_count > 0 {
                    result.push_str(&empty_count.to_string());
                    empty_count = 0;
                }
                result.push('/');
            }

            for col in 0..8 {
                match position.chessboard().squares()[row][col] {
                    square::Square::Empty => empty_count += 1,
                    square::Square::NonEmpty(piece) => {
                        if empty_count > 0 {
                            result.push_str(&empty_count.to_string());
                            empty_count = 0;
                        }
                        result.push(square_to_char(piece));
                    }
                }
            }

            if empty_count > 0 {
                result.push_str(&empty_count.to_string());
                empty_count = 0;
            }
        }

        result.push(' ');

        result.push_str(match position.status().player_turn() {
            square::Color::White => "w",
            square::Color::Black => "b",
        });

        result.push(' ');

        let mut castling = String::new();
        if position.status().castling_white_king_side() {
            castling.push('K');
        }
        if position.status().castling_white_queen_side() {
            castling.push('Q');
        }
        if position.status().castling_black_king_side() {
            castling.push('k');
        }
        if position.status().castling_black_queen_side() {
            castling.push('q');
        }
        if castling.is_empty() {
            castling.push('-');
        }
        result.push_str(&castling);

        result.push(' ');

        match position.status().pawn_en_passant() {
            Some(coord) => result.push_str(&format!("{}{}", coord.col, coord.row)),
            None => result.push('-'),
        }

        result.push(' ');
        result.push_str(&position.status().n_half_moves().to_string());
        result.push(' ');
        result.push_str(&position.status().n_moves().to_string());

        Ok(result)
    }
}

// Helper function for converting a Square to a FEN character
fn square_to_char(piece: square::Piece) -> char {
    match (piece.type_piece(), piece.color()) {
        (square::TypePiece::Rook, square::Color::White) => 'R',
        (square::TypePiece::Knight, square::Color::White) => 'N',
        (square::TypePiece::Bishop, square::Color::White) => 'B',
        (square::TypePiece::Queen, square::Color::White) => 'Q',
        (square::TypePiece::King, square::Color::White) => 'K',
        (square::TypePiece::Pawn, square::Color::White) => 'P',
        (square::TypePiece::Rook, square::Color::Black) => 'r',
        (square::TypePiece::Knight, square::Color::Black) => 'n',
        (square::TypePiece::Bishop, square::Color::Black) => 'b',
        (square::TypePiece::Queen, square::Color::Black) => 'q',
        (square::TypePiece::King, square::Color::Black) => 'k',
        (square::TypePiece::Pawn, square::Color::Black) => 'p',
    }
}

// Helper function for converting a character to a Square
fn char_to_square(c: char) -> Option<square::Square> {
    let black = square::Color::Black;
    let white = square::Color::White;
    match c {
        'R' => Some(square::Square::build_piece(square::TypePiece::Rook, white)),
        'N' => Some(square::Square::build_piece(
            square::TypePiece::Knight,
            white,
        )),
        'B' => Some(square::Square::build_piece(
            square::TypePiece::Bishop,
            white,
        )),
        'Q' => Some(square::Square::build_piece(square::TypePiece::Queen, white)),
        'K' => Some(square::Square::build_piece(square::TypePiece::King, white)),
        'P' => Some(square::Square::build_piece(square::TypePiece::Pawn, white)),
        'r' => Some(square::Square::build_piece(square::TypePiece::Rook, black)),
        'n' => Some(square::Square::build_piece(
            square::TypePiece::Knight,
            black,
        )),
        'b' => Some(square::Square::build_piece(
            square::TypePiece::Bishop,
            black,
        )),
        'q' => Some(square::Square::build_piece(square::TypePiece::Queen, black)),
        'k' => Some(square::Square::build_piece(square::TypePiece::King, black)),
        'p' => Some(square::Square::build_piece(square::TypePiece::Pawn, black)),
        '1'..='8' => None, // Empty squares will be handled by the caller (decode function)
        _ => None,         // Invalid character
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_starting_position() {
        let position = Position::build_initial_position();

        // Check if the pieces are in the correct initial positions
        assert_eq!(
            position.chessboard().squares()[0][0],
            square::Square::build_piece(TypePiece::Rook, Color::White)
        );
        assert_eq!(
            position.chessboard().squares()[0][7],
            square::Square::build_piece(TypePiece::Rook, Color::White)
        );
        assert_eq!(
            position.chessboard().squares()[7][0],
            square::Square::build_piece(TypePiece::Rook, Color::Black)
        );
        assert_eq!(
            position.chessboard().squares()[7][7],
            square::Square::build_piece(TypePiece::Rook, Color::Black)
        );

        // Check player turn
        assert_eq!(position.status().player_turn(), square::Color::White);

        // Check castling rights
        assert!(position.status().castling_white_king_side());
        assert!(position.status().castling_white_queen_side());
        assert!(position.status().castling_black_king_side());
        assert!(position.status().castling_black_queen_side());

        // Check en passant
        assert_eq!(position.status().pawn_en_passant(), None);

        // Check move counters
        assert_eq!(position.status().n_half_moves(), 0);
        assert_eq!(position.status().n_moves(), 1);
    }

    use square::Color;
    use square::Square;
    use square::TypePiece;

    #[test]
    fn test_encode_starting_position() {
        let position = Position {
            chessboard: ChessBoard::build([
                [
                    Square::build_piece(TypePiece::Rook, Color::White),
                    Square::build_piece(TypePiece::Knight, Color::White),
                    Square::build_piece(TypePiece::Bishop, Color::White),
                    Square::build_piece(TypePiece::Queen, Color::White),
                    Square::build_piece(TypePiece::King, Color::White),
                    Square::build_piece(TypePiece::Bishop, Color::White),
                    Square::build_piece(TypePiece::Knight, Color::White),
                    Square::build_piece(TypePiece::Rook, Color::White),
                ],
                [
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                    Square::build_piece(TypePiece::Pawn, Color::White),
                ],
                [
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                ],
                [
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                ],
                [
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                ],
                [
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                    Square::Empty,
                ],
                [
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                    Square::build_piece(TypePiece::Pawn, Color::Black),
                ],
                [
                    Square::build_piece(TypePiece::Rook, Color::Black),
                    Square::build_piece(TypePiece::Knight, Color::Black),
                    Square::build_piece(TypePiece::Bishop, Color::Black),
                    Square::build_piece(TypePiece::Queen, Color::Black),
                    Square::build_piece(TypePiece::King, Color::Black),
                    Square::build_piece(TypePiece::Bishop, Color::Black),
                    Square::build_piece(TypePiece::Knight, Color::Black),
                    Square::build_piece(TypePiece::Rook, Color::Black),
                ],
            ]),
            status: PositionStatus {
                castling_white_king_side: true,
                castling_white_queen_side: true,
                castling_black_king_side: true,
                castling_black_queen_side: true,
                player_turn: Color::White,
                pawn_en_passant: None,
                n_half_moves: 0,
                n_moves: 1,
            },
        };

        let fen = Fen::encode(&position).expect("Failed to encode position");
        assert_eq!(fen, FEN_START_POSITION);
    }

    #[test]
    fn test_decode_encode_symmetry() {
        let fen = FEN_START_POSITION;
        let position = Fen::decode(fen).expect("Failed to decode FEN");
        let encoded_fen = Fen::encode(&position).expect("Failed to encode position");

        assert_eq!(fen, encoded_fen);
    }

    #[test]
    fn test_decode_invalid_fen() {
        let invalid_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPP/RNBQKBNR w KQkq - 0 1"; // Missing a pawn in the last row
        let result = Fen::decode(invalid_fen);

        assert!(result.is_err());
    }

    #[test]
    fn test_encode_empty_position() {
        let empty_position = Position {
            chessboard: ChessBoard::new(),
            status: PositionStatus {
                castling_white_king_side: false,
                castling_white_queen_side: false,
                castling_black_king_side: false,
                castling_black_queen_side: false,
                player_turn: Color::White,
                pawn_en_passant: None,
                n_half_moves: 0,
                n_moves: 1,
            },
        };

        let fen = Fen::encode(&empty_position).expect("Failed to encode position");
        assert_eq!(fen, "8/8/8/8/8/8/8/8 w - - 0 1");
    }
}

use crate::ui::board::ChessBoard;
use coord::Coord;

#[test]
fn test_decode_en_passant() {
    let fen = "8/8/8/8/4p3/8/3P4/8 w - e6 0 1"; // White pawn at d2 moved to d4, black pawn on e5 can capture en passant
    let position = Fen::decode(fen).expect("Failed to decode FEN");

    // Check if en passant is set correctly
    let expected_coord = Coord::from('e', 6).expect("Failed to create Coord");
    assert_eq!(position.status().pawn_en_passant(), Some(expected_coord));
}

#[test]
fn test_encode_en_passant() {
    let position = Position {
        chessboard: ChessBoard::new(),
        status: PositionStatus {
            castling_white_king_side: false,
            castling_white_queen_side: false,
            castling_black_king_side: false,
            castling_black_queen_side: false,
            player_turn: square::Color::White,
            pawn_en_passant: Some(Coord::from('e', 6).expect("Failed to create Coord")),
            n_half_moves: 0,
            n_moves: 1,
        },
    };

    let fen = Fen::encode(&position).expect("Failed to encode position");
    assert!(
        fen.contains("E6"),
        "Expected en passant square 'e6' not found in encoded FEN"
    );
}
