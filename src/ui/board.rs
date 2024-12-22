use crate::entity::game::component::{coord, square};

#[derive(Debug, Copy, Clone)]
pub struct ChessBoard {
    squares: [[square::Square; 8]; 8],
}
pub struct ChessBoardIterator<'a> {
    board: &'a ChessBoard,
    row: usize,
    col: usize,
}
impl Iterator for ChessBoardIterator<'_> {
    type Item = square::Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.row < 8 && self.col < 8 {
            let square = self.board.squares[self.row][self.col];

            self.col += 1;
            if self.col == 8 {
                self.col = 0;
                self.row += 1;
            }

            Some(square)
        } else {
            None
        }
    }
}

impl Default for ChessBoard {
    fn default() -> Self {
        Self {
            squares: [[square::Square::Empty; 8]; 8],
        }
    }
}

impl ChessBoard {
    #[cfg(test)]
    fn at(&self, coord: coord::Coord) -> square::Square {
        self.squares[coord.get_y()][coord.get_x()]
    }

    pub fn build(squares: [[square::Square; 8]; 8]) -> Self {
        ChessBoard { squares }
    }

    pub fn squares(&self) -> &[[square::Square; 8]; 8] {
        &self.squares
    }

    pub fn iter(&self) -> ChessBoardIterator {
        ChessBoardIterator {
            board: self,
            row: 0,
            col: 0,
        }
    }

    pub fn add(&mut self, coord: coord::Coord, piece: square::TypePiece, color: square::Color) {
        self.squares[coord.get_y()][coord.get_x()] = square::Square::build_piece(piece, color);
    }
}
use std::fmt;

impl fmt::Display for ChessBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  A B C D E F G H")?;
        for (i, row) in self.squares.iter().enumerate().rev() {
            write!(f, "{} ", i + 1)?;
            for (j, square) in row.iter().enumerate() {
                let background_color = if (i + j) % 2 == 0 {
                    "\x1B[40m" // White background
                } else {
                    "\x1B[108m" // Black background
                };
                let piece_color = match square {
                    square::Square::NonEmpty(piece) if piece.color() == square::Color::White => {
                        "\x1B[97m"
                    } // White foreground
                    square::Square::NonEmpty(_) => "\x1B[96m", // Black foreground
                    square::Square::Empty => "",
                };
                let display_square = format!("{}", square);
                write!(
                    f,
                    "{}{}{} \x1B[0m",
                    background_color, piece_color, display_square
                )?;
            }
            writeln!(f)?; // New line at the end of each row
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chessboard_empty() {
        let board = ChessBoard::default();

        assert_eq!(
            board.at(coord::Coord::from('A', 1).unwrap()),
            square::Square::Empty
        );
        assert_eq!(
            board.at(coord::Coord::from('H', 8).unwrap()),
            square::Square::Empty
        );
    }

    #[test]
    fn test_chessboard_add() {
        let mut board = ChessBoard::default();
        board.add(
            coord::Coord::from('A', 1).unwrap(),
            square::TypePiece::Rook,
            square::Color::White,
        );

        assert_eq!(
            board.at(coord::Coord::from('A', 1).unwrap()),
            square::Square::build_piece(square::TypePiece::Rook, square::Color::White)
        );
    }
}
