use super::bitboard;
use super::bitboard::BitBoardMove;
use super::fen;
use super::fen::EncodeUserInput;
use super::square;

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
    use bitboard::piece_move::{CheckStatus, GenMoves, PieceMoves};
    use fen::FEN;
    use square::TypePiece;

    use super::*;

    #[test]
    fn test_game_play_castle() {
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
        let mut m: Vec<(u8, u8)> = moves.iter().map(|v| (v.start(), v.end())).collect();
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
        let short_castle_move: Vec<&BitBoardMove> = moves
            .iter()
            .filter(|v| {
                v.type_piece() == TypePiece::King && v.start() < v.end() && v.end() - v.start() == 2
            })
            .collect();
        let short_castle = *short_castle_move.get(0).unwrap();
        assert_eq!((short_castle.start(), short_castle.end()), (4u8, 6u8));
        let expected =
            BitBoardMove::new(square::Color::White, TypePiece::King, 4u8, 6u8, None, None);
        let bit_board_move = *short_castle;
        assert_eq!(bit_board_move, expected);
        let bit_board_position2 = bit_board_position.move_piece(&bit_board_move);
        let position = bit_board_position2.to();
        let fen = FEN::encode(&position).expect("Failed to encode position");
        println!("{}", position.chessboard());
        assert_eq!(fen, "qnbbkbnn/8/8/8/8/8/8/R4RK1 b kq - 1 1");
    }

    #[test]
    fn test_game_play_promotion() {
        let fen = "7k/P7/8/8/8/8/8/7K w - - 0 1";
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
        let promotion_moves: Vec<&BitBoardMove> = moves
            .iter()
            .filter(|m| m.type_piece() == TypePiece::Pawn)
            .collect();
        let new_pieces: Vec<TypePiece> =
            promotion_moves.iter().flat_map(|p| p.promotion()).collect();
        assert_eq!(new_pieces.len(), 4);
        let promotion_move = promotion_moves.get(0).unwrap();
        let bit_board_position2 = bit_board_position.move_piece(&promotion_move);
        let position = bit_board_position2.to();
        let fen = FEN::encode(&position).expect("Failed to encode position");
        println!("{}", position.chessboard());
        println!("{}", fen);
        assert_eq!(fen, "R6k/8/8/8/8/8/8/7K b - - 0 1");
    }
}
