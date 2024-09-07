#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypePiece {
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
    Pawn,
}

pub trait Switch {
    fn switch(&self) -> Color;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    White,
    Black,
}

impl Switch for Color {
    fn switch(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}



#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Piece {
    type_piece: TypePiece,
    color: Color,
}
impl Piece {
    pub fn type_piece(&self) -> TypePiece {
        self.type_piece
    }

    pub fn color(&self) -> Color {
        self.color
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Square {
    Empty,
    NonEmpty(Piece),
}

impl Square {
    pub fn build_piece(type_piece: TypePiece, color: Color) -> Square {
        Square::NonEmpty(Piece { color, type_piece })
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
            Square::NonEmpty(Piece { type_piece, color }) => {
                let symbol = match (type_piece, color) {
                    (TypePiece::Rook, Color::Black) => "♖",
                    (TypePiece::Knight, Color::Black) => "♘",
                    (TypePiece::Bishop, Color::Black) => "♗",
                    (TypePiece::Queen, Color::Black) => "♕",
                    (TypePiece::King, Color::Black) => "♔",
                    (TypePiece::Pawn, Color::Black) => "♙",
                    (TypePiece::Rook, Color::White) => "♜",
                    (TypePiece::Knight, Color::White) => "♞",
                    (TypePiece::Bishop, Color::White) => "♝",
                    (TypePiece::Queen, Color::White) => "♛",
                    (TypePiece::King, Color::White) => "♚",
                    (TypePiece::Pawn, Color::White) => "♟︎",
                };
                write!(f, "{}", symbol)
            }
        }
    }
}
