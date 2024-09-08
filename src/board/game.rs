use super::bitboard;
use super::bitboard::BitBoardMove;
use super::fen;
use super::fen::EncodeUserInput;
use super::square;
//use bitboard::piece_move::GenMoves;

pub struct Game {
    bit_position: bitboard::BitPosition,
}
impl Game {
    fn from(position: fen::Position) -> Self {
        Game {
            bit_position: bitboard::BitPosition::from(position),
        }
    }
    fn play(self, b_move: BitBoardMove) -> Game {
        Game {
            bit_position: self.bit_position.move_piece(&b_move),
        }
    }
}

#[cfg(test)]
mod tests {
    use bitboard::piece_move::{CheckStatus, GenMoves};
    use fen::FEN;
    use square::TypePiece;

    use super::*;

    #[test]
    fn test_game_play() {
        let fen = "qnbbkbnn/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
        let position = fen::FEN::decode(fen).expect("Failed to decode FEN");
        let bit_board_position = bitboard::BitPosition::from(position);
        let color = square::Color::White;
        let moves = bit_board_position
            .bit_boards_white_and_black()
            .gen_moves_for_all(
                &color,
                CheckStatus::NoCheck,
                &None,
                bit_board_position.bit_position_status(),
            );
        let mut m_all: Vec<(square::TypePiece, u8, u8)> = vec![];
        for piece_moves in &moves {
            for b in piece_moves.moves().iter() {
                m_all.push((piece_moves.type_piece(), piece_moves.index(), b))
            }
        }
        let mut m: Vec<(u8, u8)> = m_all.iter().map(|v| (v.1, v.2)).collect();
        let mut expected = vec![
            (0, 1),
            (0, 2),
            (0, 3),
            (0, 8),
            (0, 16),
            (0, 24),
            (0, 32),
            (0, 40),
            (0, 48),
            (0, 56),
            (7, 5),
            (7, 6),
            (7, 15),
            (7, 23),
            (7, 31),
            (7, 39),
            (7, 47),
            (7, 55),
            (7, 63),
            (4, 2),
            (4, 3),
            (4, 5),
            (4, 6),
            (4, 11),
            (4, 12),
            (4, 13),
        ];
        m.sort();
        expected.sort();
        assert_eq!(m, expected);
        let short_castle: Vec<&(TypePiece, u8, u8)> = m_all
            .iter()
            .filter(|v| v.0 == TypePiece::King && v.1 < v.2 && v.2 - v.1 == 2)
            .collect();
        let short_castle = short_castle.get(0).unwrap();
        assert_eq!(*short_castle, &(TypePiece::King, 4u8, 6u8));
        let bit_board_move = BitBoardMove::from(
            square::Color::White,
            short_castle.0,
            short_castle.1,
            short_castle.2,
            &bit_board_position,
        );
        let expected = BitBoardMove::new(square::Color::White, TypePiece::King, 4u8, 6u8, None);
        println!("{:?}", bit_board_move);
        assert_eq!(bit_board_move, expected);
        let bit_board_position2 = bit_board_position.move_piece(&bit_board_move);
        let position = bit_board_position2.to();
        let fen = FEN::encode(&position).expect("Failed to encode position");
        println!("{}", position.chessboard());
        assert_eq!(fen, "qnbbkbnn/8/8/8/8/8/8/R5KR b kq - 1 1");
    }
}
