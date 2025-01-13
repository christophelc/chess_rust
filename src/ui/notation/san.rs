use crate::entity::game::component::{bitboard::{self, zobrist}, game_state, square};

use super::long_notation;

#[derive(Clone)]
pub enum Lang {
    LangFr = 0,
    LangEn = 1,
}
const SAN_ROOK: [char; 2] = ['T', 'R'];
const SAN_BISHOP: [char; 2] = ['F', 'B'];
const SAN_KNIGHT: [char; 2] = ['C', 'N'];
const SAN_QUEEN: [char; 2] = ['D', 'Q'];
const SAN_KING: [char; 2] = ['R', 'K'];

#[derive(Debug)]
pub struct San {
    info: String,
}

impl San {
    pub fn new(info: String) -> Self {
        San { info }
    }
    pub fn info(&self) -> &String {
        &self.info
    }
}

fn col_as_char(col: u8) -> char {
    (b'a' + col) as char
}
fn as_str(row: u8, col: u8, capture: String) -> String {
    format!("{}{}{}", capture, col_as_char(col), (row + 1))
}

fn piece_to_char(type_piece: square::TypePiece, lang: &Lang) -> Option<char> {
    let language = lang.clone() as usize;
    match type_piece {
        square::TypePiece::Rook => Some(SAN_ROOK[language]),
        square::TypePiece::Bishop => Some(SAN_BISHOP[language]),
        square::TypePiece::Knight => Some(SAN_KNIGHT[language]),
        square::TypePiece::Queen => Some(SAN_QUEEN[language]),
        square::TypePiece::King => Some(SAN_KING[language]),
        square::TypePiece::Pawn => None,
    }
}

pub fn san_to_long_notation(san_str: &str, moves: &Vec<bitboard::BitBoardMove>, lang: &Lang, game: &game_state::GameState, zobrist_table: &zobrist::Zobrist,) -> Option<String> {
    let raw_move: Vec<_> = moves.into_iter()
    .filter(|b_move| san_to_str(b_move, moves, lang, game, zobrist_table).info() == san_str)
    .map(|b_move| long_notation::LongAlgebricNotationMove::build_from_b_move(*b_move).cast())
    .collect();
    raw_move.first().cloned()
}

fn is_move_check(game: &game_state::GameState, zobrist_table: &zobrist::Zobrist, b_move: &bitboard::BitBoardMove) -> bool {
    let mv = vec![long_notation::LongAlgebricNotationMove::build_from_b_move(*b_move)];    
    let mut game_clone = game.clone();
    game_clone.play_moves(&mv, zobrist_table, None, true).unwrap();
    game_clone.check_status().is_check()
}
pub fn san_to_str(move_to_translate: &bitboard::BitBoardMove,
    moves: &Vec<bitboard::BitBoardMove>,
    lang: &Lang,
    game: &game_state::GameState,
    zobrist_table: &zobrist::Zobrist,
) -> San {
    let check = match is_move_check(game, zobrist_table, move_to_translate) {
        true => "+",
        false => ""
    };
    let san = san_to_str_no_check(move_to_translate, moves, lang);
    San::new(format!("{}{}", san.info(), check))
}
// we need all the moves to detect ambiguous short notation moves
fn san_to_str_no_check(
    move_to_translate: &bitboard::BitBoardMove,
    moves: &Vec<bitboard::BitBoardMove>,
    lang: &Lang,
) -> San {
    let from = move_to_translate.start();
    let to = move_to_translate.end();
    match move_to_translate.check_castle() {
        Some(bitboard::Castle::Short) => return San::new("o-o".to_string()),
        Some(bitboard::Castle::Long) => return San::new("o-o-o".to_string()),
        None => {}
    };
    // look for a move that have the same destination for the same type_piece
    let moves_to: Vec<&bitboard::BitBoardMove> = (*moves)
        .iter()
        .filter(|m| {
            m.end() == to
                && m.start() != move_to_translate.start()
                && m.type_piece() == move_to_translate.type_piece()
        })
        .collect();
    let capture_as_str =
        if move_to_translate.capture().is_some() || move_to_translate.is_capture_en_passant() {
            if move_to_translate.type_piece() == square::TypePiece::Pawn {
                format!("{}x", col_as_char(move_to_translate.start().col()))
            } else {
                "x".to_string()
            }
        } else {
            "".to_string()
        };
    let to_as_str = as_str(to.row(), to.col(), capture_as_str);
    let piece_char: Option<char> = piece_to_char(move_to_translate.type_piece(), lang);
    let piece_as_str = piece_char.map_or(String::new(), |c| c.to_ascii_uppercase().to_string());
    let str = if move_to_translate.type_piece() != square::TypePiece::Pawn {
        if let Some(another_move) = moves_to.first() {
            if another_move.start().col() != from.col() {
                // We have to specify the start column or row to remove any ambiguity
                format!("{}{}{}", piece_as_str, col_as_char(from.col()), to_as_str)
            } else {
                // We have to specify the row to remove any ambiguity
                format!("{}{}{}", piece_as_str, (from.row() + 1), to_as_str)
            }
        } else {
            format!("{}{}", piece_as_str, to_as_str)
        }
    } else {
        let promotion = if let Some(new_piece) = move_to_translate.promotion() {
            format!(
                "={}",
                piece_to_char(new_piece.as_type_piece(), lang).unwrap()
            )
        } else {
            "".to_string()
        };
        format!("{}{}{}", piece_as_str, to_as_str, promotion)
    };
    San::new(str)
}

#[cfg(test)]
mod tests {
    use crate::{entity::game::component::{
        bitboard::{self, zobrist}, game_state, square::{self, TypePiecePromotion}
    }, ui::notation::{fen::{self, EncodeUserInput}, long_notation}};
    use crate::ui::notation::san::{san_to_str_no_check, Lang};

    use super::san_to_long_notation;

    #[test]
    fn test_san_pawn_capture() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Pawn;
        let start = bitboard::BitIndex::new(13);
        let end = bitboard::BitIndex::new(20);
        let capture: Option<square::TypePiece> = None;
        let promotion: Option<square::TypePiecePromotion> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str_no_check(&move_to_translate, &moves, &Lang::LangEn);
        assert_eq!(result.info(), "fxe3")
    }

    #[test]
    fn test_san_pawn_2x() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Pawn;
        let start = bitboard::BitIndex::new(13);
        let end = bitboard::BitIndex::new(29);
        let capture: Option<square::TypePiece> = None;
        let promotion: Option<square::TypePiecePromotion> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str_no_check(&move_to_translate, &moves, &Lang::LangFr);
        assert_eq!(result.info(), "f4")
    }

    #[test]
    fn test_san_pawn_promotion() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Pawn;
        let start = bitboard::BitIndex::new(48);
        let end = bitboard::BitIndex::new(56);
        let capture: Option<square::TypePiece> = None;
        let promotion: Option<TypePiecePromotion> = Some(square::TypePiecePromotion::Queen);
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str_no_check(&move_to_translate, &moves, &Lang::LangFr);
        assert_eq!(result.info(), "a8=D")
    }
    #[test]
    fn test_san_rook_capture() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Rook;
        let start = bitboard::BitIndex::new(13);
        let end = bitboard::BitIndex::new(29);
        let capture: Option<square::TypePiece> = Some(square::TypePiece::Queen);
        let promotion: Option<square::TypePiecePromotion> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str_no_check(&move_to_translate, &moves, &Lang::LangFr);
        assert_eq!(result.info(), "Txf4")
    }
    #[test]
    fn test_san_short_castle() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::King;
        let start = bitboard::BitIndex::new(4);
        let end = bitboard::BitIndex::new(6);
        let capture: Option<square::TypePiece> = None;
        let promotion: Option<square::TypePiecePromotion> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str_no_check(&move_to_translate, &moves, &Lang::LangFr);
        assert_eq!(result.info(), "o-o")
    }
    #[test]
    fn test_san_long_castle() {
        let color = square::Color::Black;
        let type_piece = square::TypePiece::King;
        let start = bitboard::BitIndex::new(60);
        let end = bitboard::BitIndex::new(58);
        let capture: Option<square::TypePiece> = None;
        let promotion: Option<square::TypePiecePromotion> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str_no_check(&move_to_translate, &moves, &Lang::LangFr);
        assert_eq!(result.info(), "o-o-o")
    }

    #[test]
    fn test_san_to_long_notation() {
        let fen_pos = "r3kbnr/ppp1pppp/2n5/8/3P1B2/2P2N2/qP3PPP/1R1QKB1R b Kkq - 1 7";
        let position = fen::Fen::decode(fen_pos).expect("Failed to decode FEN");
        let zobrist_table = zobrist::Zobrist::new();
        let mut game = game_state::GameState::new(position, &zobrist_table);
        let moves = game.gen_moves();
        // black long castle
        assert_eq!(san_to_long_notation("o-o-o", &moves, &Lang::LangFr, &game, &zobrist_table), Some("e8c8".to_string()));
        // Knight in f6
        assert_eq!(san_to_long_notation("Cf6", &moves, &Lang::LangFr, &game, &zobrist_table), Some("g8f6".to_string()));        
        // Rook in d8
        assert_eq!(san_to_long_notation("Td8", &moves, &Lang::LangFr, &game, &zobrist_table), Some("a8d8".to_string()));        
        // Pawn in e6
        assert_eq!(san_to_long_notation("e6", &moves, &Lang::LangFr, &game, &zobrist_table), Some("e7e6".to_string()));                
        let mv = vec![long_notation::LongAlgebricNotationMove::build_from_str("e7e6").unwrap()];
        game.play_moves(&mv, &zobrist_table, None, false).unwrap();
        let moves = game.gen_moves();
        // White Bishop take pawn Bxc7
        assert_eq!(san_to_long_notation("Fxc7", &moves, &Lang::LangFr, &game, &zobrist_table), Some("f4c7".to_string()));
        // White Bishop to b5
        assert_eq!(san_to_long_notation("Fb5", &moves, &Lang::LangFr, &game, &zobrist_table), Some("f1b5".to_string()));
        // White pawn to h4
        assert_eq!(san_to_long_notation("h4", &moves, &Lang::LangFr, &game, &zobrist_table), Some("h2h4".to_string()));
        let mv = vec![long_notation::LongAlgebricNotationMove::build_from_str("f1c4").unwrap()];
        game.play_moves(&mv, &zobrist_table, None, false).unwrap();
        let moves = game.gen_moves();        
        // Right Black knight to d7
        assert_eq!(san_to_long_notation("Cge7", &moves, &Lang::LangFr, &game, &zobrist_table), Some("g8e7".to_string()));
        let mv = vec![long_notation::LongAlgebricNotationMove::build_from_str("g8e7").unwrap()];
        game.play_moves(&mv, &zobrist_table, None, false).unwrap();
        let moves = game.gen_moves();        
        // white short castle
        assert_eq!(san_to_long_notation("o-o", &moves, &Lang::LangFr, &game, &zobrist_table), Some("e1g1".to_string()));
        // Bishop take queen
        assert_eq!(san_to_long_notation("Fxa2", &moves, &Lang::LangFr, &game, &zobrist_table), Some("c4a2".to_string()));
        let mv = vec![long_notation::LongAlgebricNotationMove::build_from_str("c4b5").unwrap()];        
        game.play_moves(&mv, &zobrist_table, None, false).unwrap();
        let moves = game.gen_moves();        
        // pawn
        assert_eq!(san_to_long_notation("g6", &moves, &Lang::LangFr, &game, &zobrist_table), Some("g7g6".to_string()));
        let mv = vec![long_notation::LongAlgebricNotationMove::build_from_str("g7g6").unwrap()];        
        game.play_moves(&mv, &zobrist_table, None, false).unwrap();
        let moves = game.gen_moves();        
        assert_eq!(san_to_long_notation("Fxc6+", &moves, &Lang::LangFr, &game, &zobrist_table), Some("b5c6".to_string()));
    }
}
