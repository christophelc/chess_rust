use std::collections::HashMap;
use std::fmt;

use crate::{
    entity::game::component::{
        bitboard::{self, zobrist},
        square,
    },
    ui::notation::long_notation,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PreOrder {
    Depth,
    Capture { delta: i32 },
    Mat { defender_color: square::Color },
    PreviousScore(Score),
    Promotion(square::TypePiecePromotion),
}
impl PreOrder {
    pub fn new_mat(defender_color: square::Color) -> Self {
        PreOrder::Mat { defender_color }
    }
    fn promotion_value(promotion: &square::TypePiecePromotion) -> u8 {
        match promotion {
            square::TypePiecePromotion::Queen => 5,
            square::TypePiecePromotion::Rook => 4,
            square::TypePiecePromotion::Knight => 3,
            square::TypePiecePromotion::Bishop => 3,
        }
    }
}

#[derive(Default)]
pub struct TranspositionScore {
    table: HashMap<zobrist::ZobristHash, BitboardMoveScore>,
}
impl TranspositionScore {
    pub fn get_move_score(
        &self,
        hash: &zobrist::ZobristHash,
        depth: u8,
    ) -> Option<BitboardMoveScore> {
        let mut move_score_opt: Option<BitboardMoveScore> = None;
        if let Some(move_score) = self.table.get(hash) {
            if move_score.score().path_length() >= depth {
                move_score_opt = Some(move_score.clone());
            }
        }
        move_score_opt
    }
    // store score for White
    pub fn set_move_score(&mut self, hash: &zobrist::ZobristHash, move_score: &BitboardMoveScore) {
        // ipdate only if more accurate
        if let Some(v) = self.table.get_key_value(hash) {
            // less accurate ?
            if v.1.score().path_length() < move_score.score().path_length()
                || v.1.score().path_length() == move_score.score().path_length() &&
            // capture can occur at max_depth for max_depth = 3 but not for max_depth = 2 (for example)
            v.1.score().current_depth() < move_score.score().current_depth()
            {
                return;
            }
        }
        self.table.insert(hash.clone(), move_score.clone());
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MoveStatus {
    b_move: bitboard::BitBoardMove,
    score_opt: Option<Score>,
    variant: String,
}
impl fmt::Display for MoveStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = long_notation::LongAlgebricNotationMove::build_from_b_move(self.b_move);
        match &self.score_opt {
            Some(score) => write!(f, "{}/{}:{}", m.cast(), self.get_variant(), score),
            None => write!(
                f,
                "not_evaluated({})",
                long_notation::LongAlgebricNotationMove::build_from_b_move(self.b_move).cast()
            ),
        }
    }
}
impl MoveStatus {
    pub fn from_move(b_move: bitboard::BitBoardMove) -> Self {
        Self {
            b_move,
            score_opt: None,
            variant: "".to_string(),
        }
    }
    pub fn get_move(&self) -> &bitboard::BitBoardMove {
        &self.b_move
    }
    pub fn reset_score(&mut self) {
        self.score_opt = None;
    }
    pub fn get_score(&self) -> Option<&Score> {
        self.score_opt.as_ref()
    }
    pub fn get_bitboard_move_score(&self) -> Option<BitboardMoveScore> {
        self.score_opt
            .as_ref()
            .map(|score| BitboardMoveScore::new(self.b_move, score.clone(), self.variant.clone()))
    }
    pub fn set_score(&mut self, score: Score) {
        self.score_opt = Some(score)
    }
    pub fn get_variant(&self) -> String {
        self.variant.to_string()
    }
    pub fn set_variant(&mut self, variant: &str) {
        self.variant = variant.to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BitboardMoveScoreMat {
    bitboard_move: bitboard::BitBoardMove,
    mat_in: u8,
    variant: String,
}
impl BitboardMoveScoreMat {
    pub fn new(bitboard_move: bitboard::BitBoardMove, mat_in: u8, variant: &str) -> Self {
        Self {
            bitboard_move,
            mat_in,
            variant: variant.to_string(),
        }
    }
    pub fn mat_in(&self) -> u8 {
        self.mat_in
    }
    pub fn bitboard_move(&self) -> &bitboard::BitBoardMove {
        &self.bitboard_move
    }
    pub fn variant(&self) -> String {
        self.variant.clone()
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct BitboardMoveScore {
    bitboard_move: bitboard::BitBoardMove,
    score: Score,
    variant: String,
}
impl BitboardMoveScore {
    pub fn new(bitboard_move: bitboard::BitBoardMove, score: Score, variant: String) -> Self {
        Self {
            bitboard_move,
            score,
            variant: variant.to_string(),
        }
    }
    pub fn score(&self) -> &Score {
        &self.score
    }
    pub fn bitboard_move(&self) -> &bitboard::BitBoardMove {
        &self.bitboard_move
    }
    pub fn get_variant(&self) -> String {
        self.variant.to_string()
    }
    pub fn set_variant(&mut self, variant: &str) {
        self.variant = variant.to_string();
    }
}
impl fmt::Display for BitboardMoveScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let notation =
            long_notation::LongAlgebricNotationMove::build_from_b_move(self.bitboard_move);
        write!(
            f,
            "{}:{} current_depth {} / max_depth {}",
            notation.cast(),
            self.score.value,
            self.score.current_depth(),
            self.score.max_depth
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    value: i32,
    current_depth: u8,
    max_depth: u8,
}
impl Score {
    pub fn new(value: i32, current_depth: u8, max_depth: u8) -> Self {
        Self {
            value,
            current_depth,
            max_depth,
        }
    }
    pub fn is_greater_than(&self, score: &Score) -> bool {
        self.value > score.value
            || self.value == score.value && self.path_length() > score.path_length()
    }
    pub fn is_less_than(&self, score: &Score) -> bool {
        self.value < score.value
            || self.value == score.value && self.path_length() > score.path_length()
    }
    pub fn current_depth(&self) -> u8 {
        self.current_depth
    }
    pub fn max_depth(&self) -> u8 {
        self.max_depth
    }
    pub fn path_length(&self) -> u8 {
        self.max_depth - self.current_depth
    }
    pub fn value(&self) -> i32 {
        self.value
    }
}
impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - depth: {}/{}",
            self.value, self.current_depth, self.max_depth
        )
    }
}
// better move first
pub fn compare(a: &BitboardMoveScore, b: &BitboardMoveScore) -> std::cmp::Ordering {
    b.score.value.cmp(&a.score.value)
}
pub fn find_max(vec: &[BitboardMoveScore]) -> Option<&BitboardMoveScore> {
    vec.iter().max_by(|a, b| compare(a, b))
}

fn value_type_piece(type_piece: square::TypePiece) -> i32 {
    match type_piece {
        square::TypePiece::Pawn => 1,
        square::TypePiece::Knight | square::TypePiece::Bishop => 3,
        square::TypePiece::Rook => 5,
        square::TypePiece::Queen => 10,
        _ => 0,
    }
}
// first evaluate important capture by less important pieces
pub fn biased_capture(
    type_piece: square::TypePiece,
    capture_opt: Option<square::TypePiece>,
) -> i32 {
    match capture_opt {
        None => 0,
        // +1 to evaluate first a piece that takes a piece of same value and then a non capture move
        Some(capture) => value_type_piece(capture) - value_type_piece(type_piece) + 1,
    }
}
// to be called before evaluation at depth 0 by IDDFS
pub fn preorder_compare(a: &PreOrder, b: &PreOrder) -> std::cmp::Ordering {
    if a == b {
        return std::cmp::Ordering::Equal;
    }
    match (a, b) {
        // we can have only one PV: play it first
        (PreOrder::PreviousScore(sc1), PreOrder::PreviousScore(sc2)) => {
            sc2.value().cmp(&sc1.value())
        }
        (_, PreOrder::PreviousScore(_)) => std::cmp::Ordering::Greater,
        (PreOrder::PreviousScore(_), _) => std::cmp::Ordering::Less,
        (PreOrder::Promotion(pa), PreOrder::Promotion(pb)) => {
            PreOrder::promotion_value(pb).cmp(&PreOrder::promotion_value(pa))
        }
        (PreOrder::Promotion(_), _) => std::cmp::Ordering::Less,
        (_, PreOrder::Promotion(_)) => std::cmp::Ordering::Greater,
        (PreOrder::Mat { defender_color: _ }, _) => std::cmp::Ordering::Less,
        (_, PreOrder::Mat { defender_color: _ }) => std::cmp::Ordering::Greater,
        (PreOrder::Capture { delta: delta_a }, PreOrder::Capture { delta: delta_b }) => {
            delta_b.cmp(delta_a)
        }
        (PreOrder::Capture { delta }, PreOrder::Depth) => {
            if *delta >= 0 {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        }
        (PreOrder::Depth, PreOrder::Capture { delta }) => {
            if *delta >= 0 {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Less
            }
        }
        _ => std::cmp::Ordering::Equal,
    }
}
// to be called for depth >=1 by IDDFS
pub fn order_move_status(a: &MoveStatus, b: &MoveStatus) -> std::cmp::Ordering {
    match (a.get_score(), b.get_score()) {
        (Some(score_a), Some(score_b)) => score_b.value().cmp(&score_a.value()),
        // put non evaluated node at the end
        (Some(_score_a), _) => std::cmp::Ordering::Less,
        (_, Some(_score_b)) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::engine::component::score::{order_move_status, PreOrder};
    use crate::entity::game::component::{bitboard, square};

    use super::preorder_compare;
    use super::BitboardMoveScore;
    use super::MoveStatus;
    use super::Score;

    #[test]
    fn test_sort() {
        let m = bitboard::BitBoardMove::new(
            square::Color::White,
            square::TypePiece::Rook,
            bitboard::BitIndex::new(0),
            bitboard::BitIndex::new(1),
            None,
            None,
        );
        let current_depth = 0;
        let max_depth = 0;
        let moves_status1 = MoveStatus {
            b_move: m.clone(),
            variant: "1".to_string(),
            score_opt: Some(Score::new(0, current_depth, max_depth)),
        };
        let moves_status2 = MoveStatus {
            b_move: m.clone(),
            variant: "2".to_string(),
            score_opt: Some(Score::new(-5, current_depth, max_depth)),
        };
        let moves_status3 = MoveStatus {
            b_move: m.clone(),
            variant: "3".to_string(),
            score_opt: Some(Score::new(3, current_depth, max_depth)),
        };
        let moves_status4 = MoveStatus {
            b_move: m.clone(),
            variant: "4".to_string(),
            score_opt: None,
        };
        let moves_status5 = MoveStatus {
            b_move: m.clone(),
            variant: "5".to_string(),
            score_opt: Some(Score::new(-6, current_depth, max_depth)),
        };
        let moves_status6 = MoveStatus {
            b_move: m.clone(),
            variant: "6".to_string(),
            score_opt: Some(Score::new(-7, current_depth, max_depth)),
        };
        let moves_status7 = MoveStatus {
            b_move: m.clone(),
            variant: "7".to_string(),
            score_opt: Some(Score::new(-8, current_depth, max_depth)),
        };
        let moves_status8 = MoveStatus {
            b_move: m.clone(),
            variant: "8".to_string(),
            score_opt: Some(Score::new(-9, current_depth, max_depth)),
        };

        let mut v = vec![
            moves_status8.clone(),
            moves_status7.clone(),
            moves_status6.clone(),
            moves_status5.clone(),
            moves_status4.clone(),
            moves_status3.clone(),
            moves_status2.clone(),
            moves_status1.clone(),
        ];
        let expected1 = vec![
            moves_status3.clone(),
            moves_status1.clone(),
            moves_status2.clone(),
            moves_status5.clone(),
            moves_status6.clone(),
            moves_status7.clone(),
            moves_status8.clone(),
            moves_status4.clone(),
        ];
        v.sort_by(order_move_status);
        let v_variant: Vec<String> = v.iter().map(|m| m.get_variant()).collect();
        let expected1_variant: Vec<String> = expected1.iter().map(|m| m.get_variant()).collect();
        assert_eq!(v_variant, expected1_variant);
        let expected2 = vec![
            moves_status5.clone(),
            moves_status7.clone(),
            moves_status6.clone(),
            moves_status3.clone(),
            moves_status1.clone(),
            moves_status2.clone(),
            moves_status4.clone(),
            moves_status8.clone(),
        ];
    }

    //////////////////////////////////
    // Preorder test (generated by AI)
    //////////////////////////////////
    #[test]
    fn test_preorder_sorting() {
        let current_depth = 0;
        let max_depth = 0;
        let mut list = vec![
            PreOrder::PreviousScore(Score::new(10, current_depth, max_depth)),
            PreOrder::Promotion(square::TypePiecePromotion::Queen),
            PreOrder::Capture { delta: 5 },
            PreOrder::Depth,
            PreOrder::new_mat(square::Color::White),
            PreOrder::PreviousScore(Score::new(20, current_depth, max_depth)),
            PreOrder::Promotion(square::TypePiecePromotion::Rook),
            PreOrder::Capture { delta: -5 },
            PreOrder::Depth,
            PreOrder::Capture { delta: 10 },
        ];

        list.sort_by(preorder_compare);

        let expected = vec![
            PreOrder::PreviousScore(Score::new(20, current_depth, max_depth)),
            PreOrder::PreviousScore(Score::new(10, current_depth, max_depth)),
            PreOrder::Promotion(square::TypePiecePromotion::Queen),
            PreOrder::Promotion(square::TypePiecePromotion::Rook),
            PreOrder::new_mat(square::Color::White),
            PreOrder::Capture { delta: 10 },
            PreOrder::Capture { delta: 5 },
            PreOrder::Depth,
            PreOrder::Depth,
            PreOrder::Capture { delta: -5 },
        ];

        assert_eq!(list, expected);
    }
}
