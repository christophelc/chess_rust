#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Piece {
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
pub enum Square {
    Empty,
    NonEmpty {
        color: Color,
        piece: Piece,
    }
}

impl Square {
    pub fn build_piece(piece: Piece, color: Color) -> Square {
        Square::NonEmpty {
            color,
            piece
        }
    }
}

use std::fmt;

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Square::Empty => write!(f, " "),
            Square::NonEmpty { piece, color } => {
                let symbol = match (piece, color) {
                    (Piece::Rook, Color::White) => "♖",
                    (Piece::Knight, Color::White) => "♘",
                    (Piece::Bishop, Color::White) => "♗",
                    (Piece::Queen, Color::White) => "♕",
                    (Piece::King, Color::White) => "♔",
                    (Piece::Pawn, Color::White) => "♙",
                    (Piece::Rook, Color::Black) => "♜",
                    (Piece::Knight, Color::Black) => "♞",
                    (Piece::Bishop, Color::Black) => "♝",
                    (Piece::Queen, Color::Black) => "♛",
                    (Piece::King, Color::Black) => "♚",
                    (Piece::Pawn, Color::Black) => "♟︎",
                };
                write!(f, "{}", symbol)
            }
        }
    }
}
