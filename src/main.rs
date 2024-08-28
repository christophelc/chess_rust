mod board;
use board::coord;

use board::Board;

fn main() {
    let mut bd = board::ChessBoard::new();
    bd.set_initial_position();
    bd.move_piece(coord::Coord::from('E', 2).unwrap(), coord::Coord::from('E', 4).unwrap());
    println!("{}", bd);
}
