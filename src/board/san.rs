use super::{bitboard, square::TypePiece};

const LANG_FR: usize = 0;
const LANG_EN: usize = 1;
const SAN_ROOK: [char; 2] = ['T', 'R'];
const SAN_BISHOP: [char; 2] = ['F', 'B'];
const SAN_KNIGHT: [char; 2] = ['C', 'N'];
const SAN_QUEEN: [char; 2] = ['D', 'Q'];
const SAN_KING: [char; 2] = ['R', 'K'];

#[derive(Debug)]
pub struct SAN {
    info: String,
}

impl SAN {
    pub fn new(info: String) -> Self {
        SAN { info }
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

fn piece_to_char(type_piece: TypePiece, language: usize) -> Option<char> {
    match type_piece {
        TypePiece::Rook => Some(SAN_ROOK[language]),
        TypePiece::Bishop => Some(SAN_BISHOP[language]),
        TypePiece::Knight => Some(SAN_KNIGHT[language]),
        TypePiece::Queen => Some(SAN_QUEEN[language]),
        TypePiece::King => Some(SAN_KING[language]),
        TypePiece::Pawn => None,
    }
}

pub fn san_to_str(
    move_to_translate: &bitboard::BitBoardMove,
    moves: Vec<bitboard::BitBoardMove>,
    language: usize,
) -> SAN {
    assert!(
        language < 2,
        "language can be either 0 or 1. Here we have: {}",
        language
    );

    let to = move_to_translate.end();
    match move_to_translate.check_castle() {
        Some(bitboard::Castle::ShortCastle) => return SAN::new("o-o".to_string()),
        Some(bitboard::Castle::LongCastle) => return SAN::new("o-o-o".to_string()),
        None => {},
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
    let row = to / 8;
    let col = to % 8;
    let capture_as_str =
        if move_to_translate.capture().is_some() || move_to_translate.is_capture_en_passant() {
            if move_to_translate.type_piece() == TypePiece::Pawn {
                format!("{}x", col_as_char(move_to_translate.start() % 8))
            } else {
                "x".to_string()
            }
        } else {
            "".to_string()
        };
    let to_as_str = as_str(row, col, capture_as_str);
    let piece_char: Option<char> = piece_to_char(move_to_translate.type_piece(), language);
    let piece_as_str = piece_char.map_or(String::new(), |c| c.to_ascii_uppercase().to_string());
    let str = if move_to_translate.type_piece() != TypePiece::Pawn {
        if let Some(another_move) = moves_to.first().clone() {
            let row_2 = another_move.end() / 8;
            let col_2 = another_move.end() % 8;
            if row_2 == row {
                // We have to specify the column or the row to remove any ambiguity
                format!("{}{}{}", piece_as_str, col_as_char(col_2), to_as_str)
            } else {
                // We have to specify the row to remove any ambiguity
                format!("{}{}{}", piece_as_str, (row_2 + 1), to_as_str)
            }
        } else {
            format!("{}{}", piece_as_str, to_as_str)
        }
    } else {
        let promotion = if let Some(new_piece) = move_to_translate.promotion() {
            format!("={}", piece_to_char(new_piece, language).unwrap())
        } else {
            "".to_string()
        };
        format!("{}{}{}", piece_as_str, to_as_str, promotion)
    };
    SAN::new(str)
}

mod tests {
    use crate::board::square::{self, TypePiece};

    use super::*;

    #[test]
    fn test_san_pawn_capture() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Pawn;
        let start = 13;
        let end = 20;
        let capture: Option<TypePiece> = None;
        let promotion: Option<TypePiece> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str(&move_to_translate, moves, LANG_FR);
        assert_eq!(result.info(), "fxe3")
    }

    #[test]
    fn test_san_pawn_2x() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Pawn;
        let start = 13;
        let end = 29;
        let capture: Option<TypePiece> = None;
        let promotion: Option<TypePiece> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str(&move_to_translate, moves, LANG_FR);
        assert_eq!(result.info(), "f4")
    }

    #[test]
    fn test_san_pawn_promotion() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Pawn;
        let start = 48;
        let end = 56;
        let capture: Option<TypePiece> = None;
        let promotion: Option<TypePiece> = Some(TypePiece::Queen);
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str(&move_to_translate, moves, LANG_FR);
        assert_eq!(result.info(), "a8=D")
    }
    #[test]
    fn test_san_rook_capture() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::Rook;
        let start = 13;
        let end = 29;
        let capture: Option<TypePiece> = Some(TypePiece::Queen);
        let promotion: Option<TypePiece> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str(&move_to_translate, moves, LANG_FR);
        assert_eq!(result.info(), "Txf4")
    }
    #[test]
    fn test_san_short_castle() {
        let color = square::Color::White;
        let type_piece = square::TypePiece::King;
        let start = 4;
        let end = 6;
        let capture: Option<TypePiece> = None;
        let promotion: Option<TypePiece> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str(&move_to_translate, moves, LANG_FR);
        assert_eq!(result.info(), "o-o")
    }
    #[test]
    fn test_san_long_castle() {
        let color = square::Color::Black;
        let type_piece = square::TypePiece::King;
        let start = 60;
        let end = 58;
        let capture: Option<TypePiece> = None;
        let promotion: Option<TypePiece> = None;
        let move_to_translate =
            bitboard::BitBoardMove::new(color, type_piece, start, end, capture, promotion);
        let moves = vec![move_to_translate];
        let result = san_to_str(&move_to_translate, moves, LANG_FR);
        assert_eq!(result.info(), "o-o-o")
    }
}
