use std::fmt;

use crate::{
    entity::game::component::bitboard::{self, BitBoardMove},
    ui::notation::long_notation,
};

pub enum MoveStatus {
    Evaluated(BitboardMoveScore),        // Fully evaluated move with a score
    Pruned(bitboard::BitBoardMove),      // Pruned move without an evaluation
    NotEvaluated(bitboard::BitBoardMove), // Move yet to be evaluated
}
impl MoveStatus {
    pub fn get_move(&self) -> &bitboard::BitBoardMove {
        match self {
            Self::Evaluated(move_score) => move_score.bitboard_move(),
            Self::Pruned(move_pruned) => move_pruned,
            Self::NotEvaluated(move_not_evaluated) => move_not_evaluated,
        }
    }
    pub fn get_score(&self) -> Option<&Score> {
            self.get_bitboard_move_score().map(|m_score| m_score.score())
    }
    pub fn get_bitboard_move_score(&self) -> Option<&BitboardMoveScore> {
        match self {
            Self::Evaluated(move_score) => Some(move_score),
            _ => None,
        }        
    }
    pub fn into_bitboard_move_score(self) -> Option<BitboardMoveScore> {
        match self {
            Self::Evaluated(move_score) => Some(move_score),
            _ => None,
        }        
    }    
}

#[derive(Debug, Clone)]
pub struct BitboardMoveScore {
    bitboard_move: bitboard::BitBoardMove,
    score: Score,
}
impl BitboardMoveScore {
    pub fn new(bitboard_move: bitboard::BitBoardMove, score: Score) -> Self {
        Self {
            bitboard_move,
            score,
        }
    }
    pub fn score(&self) -> &Score {
        &self.score
    }
    pub fn bitboard_move(&self) -> &bitboard::BitBoardMove {
        &self.bitboard_move
    }
}
impl fmt::Display for BitboardMoveScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let notation =
            long_notation::LongAlgebricNotationMove::build_from_b_move(self.bitboard_move);
        write!(f, "{}:{}", notation.cast(), self.score.value)
    }
}

#[derive(Clone, Debug)]
pub struct Score {
    value: i32,
    path_length: u8,
}
impl Score {
    pub fn new(value: i32, path_length: u8) -> Self {
        Self { value, path_length }
    }
    pub fn is_better_than(&self, score: &Score) -> bool {
        self.value > score.value
            || self.value == score.value && self.path_length < score.path_length
    }
    pub fn opposite(&self) -> Self {
        Self {
            value: -self.value,
            path_length: self.path_length,
        }
    }
    pub fn path_length(&self) -> u8 {
        self.path_length
    }
    pub fn value(&self) -> i32 {
        self.value
    }
}
impl fmt::Display for Score {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - path length: {}", self.value, self.path_length)
    }
}
// better move first
pub fn compare(a: &BitboardMoveScore, b: &BitboardMoveScore) -> std::cmp::Ordering {
    b.score.value.cmp(&a.score.value)
}
pub fn find_max(vec: &[BitboardMoveScore]) -> Option<&BitboardMoveScore> {
    vec.iter().max_by(|a, b| compare(a, b))
}

pub fn compare_move_status(a: &MoveStatus, b: &MoveStatus) -> std::cmp::Ordering {
    match (a, b) {
        (MoveStatus::Evaluated(score_a), MoveStatus::Evaluated(score_b)) =>
            score_b.score.value().cmp(&score_a.score.value()),
        (MoveStatus::Evaluated(_score_a), _) => std::cmp::Ordering::Greater,            
        (_, MoveStatus::Evaluated(_score_b)) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal
    }
}