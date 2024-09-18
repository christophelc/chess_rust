use crate::board::{bitboard, fen::Position};
use std::io::Stdout;
use std::io::{self, Write};

use super::parameters;

#[derive(Clone, Default)]
pub struct Configuration {
    parameters: parameters::Parameters,
    opt_position: Option<Position>,
}
impl PartialEq for Configuration {
    fn eq(&self, other: &Self) -> bool {
        let equal_opt_position = match (self.opt_position, other.opt_position) {
            (None, None) => true,
            (Some(pos), Some(pos_other)) => {
                let bit_position = bitboard::BitPosition::from(pos);
                let bit_position_other = bitboard::BitPosition::from(pos_other);
                bit_position == bit_position_other
            }
            _ => false,
        };
        self.parameters == other.parameters && equal_opt_position
    }
}
impl Configuration {
    pub fn update_parameters(&mut self, parameters: parameters::Parameters) {
        self.parameters = parameters;
    }
    pub fn update_position(&mut self, position: Position) {
        self.opt_position = Some(position);
    }

    pub fn opt_position(&self) -> Option<Position> {
        self.opt_position
    }
    pub fn parameters(&self) -> &parameters::Parameters {
        &self.parameters
    }
}

pub fn write_err(stdout: &mut Stdout, err: String) -> Result<(), io::Error> {
    let res = writeln!(stdout, "{}", err);
    stdout.flush().unwrap();
    res
}
