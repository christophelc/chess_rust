use std::error::Error;
/// This module implements a TUI for a chessboard
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Coord {
    pub col: char,
    pub row: u8,
}

impl fmt::Display for Coord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.col, self.row)
    }
}

impl Coord {
    pub fn get_x(&self) -> usize {
        (self.col as u8 - 'A' as u8) as usize
    }
    pub fn get_y(&self) -> usize {
        (self.row - 1) as usize
    }
    pub fn from(col: char, row: u8) -> Result<Self, Box<dyn Error>> {
        if Self::is_valid_chess_square(col, row) {
            Ok(Coord {
                col: col.to_uppercase().next().unwrap(),
                row,
            })
        } else {
            Err(Box::new(InvalidCoordError { col, row }))
        }
    }
    fn is_valid_chess_square(col: char, row: u8) -> bool {
        match col {
            'A'..='H' | 'a'..='h' if row >= 1 && row <= 8 => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
struct InvalidCoordError {
    pub col: char,
    pub row: u8,
}
impl fmt::Display for InvalidCoordError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.col, self.row)
    }
}
impl Error for InvalidCoordError {}

//use std::env;
//use std::ffi::OsString;
//use std::process;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chess_square_from() {
        // Valid chess squares
        assert!(Coord::from('A', 1).is_ok());
        assert!(Coord::from('H', 8).is_ok());

        // Invalid chess squares
        assert!(Coord::from('I', 1).is_err());
        // Import the `assert_eq!` macro from the `assert_macros` crate to run tests.
        // Add the following line at the top of your test module:
        //use assert_macros::assert_eq;

        // Update the test case for invalid chess squares to use `assert_eq!`.
        let box_error = Coord::from('I', 1).err().unwrap();
        if let Some(error) = box_error.downcast_ref::<InvalidCoordError>() {
            assert_eq!(*error, InvalidCoordError { col: 'I', row: 1 });
        } else {
            panic!("Error is not of type InvalidCoordError");
        }
        assert!(Coord::from('A', 9).is_err());
    }

    #[test]
    fn test_is_valid_chess_square() {
        // Valid chess squares
        assert!(Coord::is_valid_chess_square('a', 1));
        assert!(Coord::is_valid_chess_square('h', 8));

        // Invalid chess squares
        assert!(!Coord::is_valid_chess_square('i', 1));
        assert!(!Coord::is_valid_chess_square('j', 8));
    }
}
