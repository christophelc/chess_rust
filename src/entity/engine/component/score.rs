use std::collections::HashMap;
use std::fmt;

use crate::{
    entity::game::component::{
        bitboard::{self, zobrist},
        square,
    },
    ui::notation::long_notation,
};

#[derive(Default)]
pub struct TranspositionScore {
    table: HashMap<zobrist::ZobristHash, BitboardMoveScore>,
}
impl TranspositionScore {
    pub fn get_move_score(
        &self,
        hash: &zobrist::ZobristHash,
        player_turn: &square::Color,
        depth: u8,
    ) -> Option<BitboardMoveScore> {
        let mut move_score_opt: Option<BitboardMoveScore> = None;
        if let Some(move_score) = self.table.get(hash) {
            if move_score.score().path_length() >= depth {
                match player_turn {
                    square::Color::White => move_score_opt = Some(move_score.clone()),
                    square::Color::Black => move_score_opt = Some(move_score.opposite()),
                }
            }
        }
        move_score_opt
    }
    // store score for White
    pub fn set_move_score(
        &mut self,
        hash: &zobrist::ZobristHash,
        player_turn: &square::Color,
        move_score: &BitboardMoveScore,
    ) {
        match player_turn {
            square::Color::White => self.table.insert(hash.clone(), move_score.clone()),
            square::Color::Black => self.table.insert(hash.clone(), move_score.opposite()),
        };
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveStatus {
    Evaluated(BitboardMoveScore),   // Fully evaluated move with a score
    Pruned(bitboard::BitBoardMove), // Pruned move without an evaluation
    NotEvaluated(bitboard::BitBoardMove), // Move yet to be evaluated
}
impl fmt::Display for MoveStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MoveStatus::Evaluated(move_score) => write!(f, "evaluated({})", move_score.to_string()),
            MoveStatus::Pruned(b_move) => write!(
                f,
                "pruned({})",
                long_notation::LongAlgebricNotationMove::build_from_b_move(*b_move).cast()
            ),
            MoveStatus::NotEvaluated(b_move) => write!(
                f,
                "not_evaluated({})",
                long_notation::LongAlgebricNotationMove::build_from_b_move(*b_move).cast()
            ),
        }
    }
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
        self.get_bitboard_move_score()
            .map(|m_score| m_score.score())
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

#[derive(Debug, Clone, PartialEq)]
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
    pub fn opposite(&self) -> BitboardMoveScore {
        BitboardMoveScore {
            bitboard_move: self.bitboard_move,
            score: self.score.opposite(),
        }
    }
    pub fn bitboard_move(&self) -> &bitboard::BitBoardMove {
        &self.bitboard_move
    }
}
impl fmt::Display for BitboardMoveScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let notation =
            long_notation::LongAlgebricNotationMove::build_from_b_move(self.bitboard_move);
        write!(
            f,
            "{}:{} depth:{}",
            notation.cast(),
            self.score.value,
            self.score.path_length()
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
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
        (MoveStatus::Evaluated(score_a), MoveStatus::Evaluated(score_b)) => {
            score_b.score.value().cmp(&score_a.score.value())
        }
        // put non evaluated node first
        (MoveStatus::Evaluated(_score_a), _) => std::cmp::Ordering::Greater,
        (_, MoveStatus::Evaluated(_score_b)) => std::cmp::Ordering::Less,
        _ => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::game::component::{bitboard, square};

    use super::compare_move_status;
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
        let path_length = 0;
        let moves_status1 = MoveStatus::Evaluated(BitboardMoveScore::new(
            m.clone(),
            Score::new(0, path_length),
        ));
        let moves_status2 = MoveStatus::Evaluated(BitboardMoveScore::new(
            m.clone(),
            Score::new(-5, path_length),
        ));
        let moves_status3 = MoveStatus::Evaluated(BitboardMoveScore::new(
            m.clone(),
            Score::new(3, path_length),
        ));
        let moves_status4 = MoveStatus::NotEvaluated(m.clone());
        let mut v = vec![
            moves_status1.clone(),
            moves_status2.clone(),
            moves_status3.clone(),
            moves_status4.clone(),
        ];
        let expected = vec![moves_status4, moves_status3, moves_status1, moves_status2];
        v.sort_by(compare_move_status);
        assert_eq!(v, expected)
    }
}
