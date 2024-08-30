pub mod coord;
pub mod square;
pub mod fen;
pub mod bitboard;

pub trait Board {
    fn set_initial_position(&mut self);

    fn add(&mut self, coord: coord::Coord, piece: square::TypePiece, color: square::Color);

    fn move_piece(&mut self, from: coord::Coord, to: coord::Coord);
}

#[derive(Debug)]
pub struct ChessBoard {
    squares: [[square::Square; 8]; 8],
}

impl ChessBoard {
    fn at(&self, coord: coord::Coord) -> square::Square {
        self.squares[coord.get_y()][coord.get_x()]
    }

    pub fn new() -> Self {
        ChessBoard::empty()
    }

    // Helper function to create an empty board
    fn empty() -> Self {
        ChessBoard {
            squares: [[square::Square::Empty; 8]; 8],
        }
    }
}

impl Board for ChessBoard {
    fn set_initial_position(&mut self) {
        // Setup an example board configuration
        self.add(coord::Coord::from('A', 1).unwrap(), square::TypePiece::Rook, square::Color::Black);
        self.add(coord::Coord::from('B', 1).unwrap(), square::TypePiece::Knight, square::Color::Black);    
        self.add(coord::Coord::from('C', 1).unwrap(), square::TypePiece::Bishop, square::Color::Black);        
        self.add(coord::Coord::from('D', 1).unwrap(), square::TypePiece::Queen, square::Color::Black);        
        self.add(coord::Coord::from('E', 1).unwrap(), square::TypePiece::King, square::Color::Black);            
        self.add(coord::Coord::from('F', 1).unwrap(), square::TypePiece::Bishop, square::Color::Black);                
        self.add(coord::Coord::from('G', 1).unwrap(), square::TypePiece::Knight, square::Color::Black);
        self.add(coord::Coord::from('H', 1).unwrap(), square::TypePiece::Rook, square::Color::Black);                    
        for col in 'A'..='H' {
            self.add(coord::Coord::from(col, 2).unwrap(), square::TypePiece::Pawn, square::Color::Black);                        
        }

        self.add(coord::Coord::from('A', 8).unwrap(), square::TypePiece::Rook, square::Color::White);
        self.add(coord::Coord::from('B', 8).unwrap(), square::TypePiece::Knight, square::Color::White);    
        self.add(coord::Coord::from('C', 8).unwrap(), square::TypePiece::Bishop, square::Color::White);        
        self.add(coord::Coord::from('D', 8).unwrap(), square::TypePiece::Queen, square::Color::White);        
        self.add(coord::Coord::from('E', 8).unwrap(), square::TypePiece::King, square::Color::White);            
        self.add(coord::Coord::from('F', 8).unwrap(), square::TypePiece::Bishop, square::Color::White);                
        self.add(coord::Coord::from('G', 8).unwrap(), square::TypePiece::Knight, square::Color::White);
        self.add(coord::Coord::from('H', 8).unwrap(), square::TypePiece::Rook, square::Color::White);                    
        for col in 'A'..='H' {
            self.add(coord::Coord::from(col, 7).unwrap(), square::TypePiece::Pawn, square::Color::White);
        }
    }

    fn add(&mut self, coord: coord::Coord, piece: square::TypePiece, color: square::Color) {
        self.squares[coord.get_y()][coord.get_x()] = square::Square::build_piece(piece, color);
    }   

    fn move_piece(&mut self, from: coord::Coord, to: coord::Coord) {
        self.squares[to.get_y()][to.get_x()] = self.at(from);
        self.squares[from.get_y()][from.get_x()] = square::Square::Empty;
    }
}
use std::fmt;

impl fmt::Display for ChessBoard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "  A B C D E F G H")?;        
        for (i, row) in self.squares.iter().enumerate().rev() {
            write!(f, "{} ", 8 - i)?;
            for (j, square) in row.iter().enumerate() {
                let color = if (i + j) % 2 == 0 {
                    "\x1B[100m" // White background
                } else {
                    "\x1B[40m" // Black background
                }; 
                let display_square = format!("{}", square);
                write!(f, "{}{} \x1B[0m", color, display_square)?;                               
            }
            writeln!(f)?; // New line at the end of each row
        }
        Ok(())
    }
}

#[test]
fn test_chessboard_empty() {
    let board = ChessBoard::new();

    assert_eq!(board.at(coord::Coord::from('A', 1).unwrap()), square::Square::Empty);
    assert_eq!(board.at(coord::Coord::from('H', 8).unwrap()), square::Square::Empty);
}

#[test]
fn test_chessboard_add() {
    let mut board = ChessBoard::new();
    board.add(coord::Coord::from('A', 1).unwrap(), square::TypePiece::Rook, square::Color::White);

    assert_eq!(board.at(coord::Coord::from('A', 1).unwrap()), square::Square::build_piece(square::TypePiece::Rook, square::Color::White));
}

#[test]
fn test_chessboard_move() {
    let mut board = ChessBoard::new();
    board.add(coord::Coord::from('A', 1).unwrap(), square::TypePiece::Rook, square::Color::White);

    board.move_piece(coord::Coord::from('A', 1).unwrap(), coord::Coord::from('B', 2).unwrap());

    assert_eq!(board.at(coord::Coord::from('A', 1).unwrap()), square::Square::Empty);
    assert_eq!(board.at(coord::Coord::from('B', 2).unwrap()), square::Square::build_piece(square::TypePiece::Rook, square::Color::White));
}

