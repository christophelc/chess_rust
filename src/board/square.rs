#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypePiece {
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
    Pawn
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    White,
    Black
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Piece {
    piece_type: TypePiece,
    color: Color,
}
impl Piece {
    pub fn piece_type(&self) -> TypePiece { self.piece_type }

    pub fn color(&self) -> Color { self.color }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Square {
    Empty,
    NonEmpty(Piece)
}

impl Square {
    pub fn build_piece(piece_type: TypePiece, color: Color) -> Square {
        Square::NonEmpty(Piece { 
            color, piece_type
        })
    }
    pub fn is_empty(&self) -> bool {
        *self == Square::Empty
    }
    pub fn non_empty(&self) -> bool {
        !self.is_empty()
    }
}

use std::fmt;

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Square::Empty => write!(f, " "),
            Square::NonEmpty(Piece { piece_type, color }) => {
                let symbol = match (piece_type, color) {
                    (TypePiece
                    ::Rook, Color::White) => "♖",
                    (TypePiece
                    ::Knight, Color::White) => "♘",
                    (TypePiece
                    ::Bishop, Color::White) => "♗",
                    (TypePiece
                    ::Queen, Color::White) => "♕",
                    (TypePiece
                    ::King, Color::White) => "♔",
                    (TypePiece
                    ::Pawn, Color::White) => "♙",
                    (TypePiece
                    ::Rook, Color::Black) => "♜",
                    (TypePiece
                    ::Knight, Color::Black) => "♞",
                    (TypePiece
                    ::Bishop, Color::Black) => "♝",
                    (TypePiece
                    ::Queen, Color::Black) => "♛",
                    (TypePiece
                    ::King, Color::Black) => "♚",
                    (TypePiece
                    ::Pawn, Color::Black) => "♟︎",
                };
                write!(f, "{}", symbol)
            }
        }
    }
}
