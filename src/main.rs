mod board;
use board::bitboard;
use board::bitboard::BitBoard;
use board::bitboard::BitPosition;
use board::coord;
use board::fen;
use board::fen::Position;
use board::square;
use board::Board;
use board::ChessBoard;
use fen::EncodeUserInput;

use board::bitboard::piece_move;
use piece_move::GenMoves;

fn main() {
    println!("Inital position with move e4");
    let mut bd = board::ChessBoard::new();
    bd.set_initial_position();
    bd.move_piece(
        coord::Coord::from('E', 2).unwrap(),
        coord::Coord::from('E', 4).unwrap(),
    );
    println!("{}", bd);
    println!();

    println!("chessboard generated from initial position encoded with FEN");
    let position: fen::Position = fen::Position::build_initial_position();
    println!("{}", position.chessboard());
    let fen_str = fen::FEN::encode(&position).expect("Error when decoding position to FEN format.");
    println!("Encode initial position to FEN position:");
    println!("{}", fen_str);

    println!("Generate moves for white king considering pawn is in e4");
    let status = position.status().clone();
    let mut bd = position.into_chessboard().clone();
    bd.move_piece(
        coord::Coord::from('E', 2).unwrap(),
        coord::Coord::from('E', 4).unwrap(),
    );
    let position = Position::build(bd, status);
    let bit_position = BitPosition::from(position);
    let moves = bit_position.bit_boards_white_and_black().gen_moves_for_all(
        &board::square::Color::White,
        piece_move::CheckStatus::NoCheck,
        &None,
        bit_position.bit_position_status(),
    );
    println!("{:?}", moves);
}
