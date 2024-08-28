mod board;
use board::coord;
use board::Board;
use board::fen;
use fen::EncodeUserInput;

fn main() {
    println!("Inital position with move e4");
    let mut bd = board::ChessBoard::new();
    bd.set_initial_position();
    bd.move_piece(coord::Coord::from('E', 2).unwrap(), coord::Coord::from('E', 4).unwrap());
    println!("{}", bd);
    println!();

    println!("chessboard generated from initial position encoded with FEN");
    let position: fen::Position = fen::Position::new();
    println!("{}", position.chessboard());
    let fen_str = fen::FEN::encode(&position).expect("Error when decoding position to FEN format.");
    println!("Encode initial position to FEN position:");
    println!("{}", fen_str)
}
