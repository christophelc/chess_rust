use crate::entity::game::component::bitboard;
use crate::ui::notation::long_notation;
use super::engine_logic as logic;
use super::ts_best_move;

#[derive(Debug, Clone)]
pub struct TimestampedBitBoardMove {
    best_move: bitboard::BitBoardMove,
    timestamp: chrono::DateTime<chrono::Utc>,
    engine_id: logic::EngineId,
}
impl TimestampedBitBoardMove {
    pub fn new(best_move: bitboard::BitBoardMove, engine_id: logic::EngineId) -> Self {
        Self {
            best_move,
            timestamp: chrono::Utc::now(),
            engine_id,
        }
    }
    pub fn best_move(&self) -> bitboard::BitBoardMove {
        self.best_move
    }
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }
    pub fn engine_id(&self) -> logic::EngineId {
        self.engine_id.clone()
    }
    pub fn to_ts_best_move(&self) -> ts_best_move::TimestampedBestMove {
        let best_move = long_notation::LongAlgebricNotationMove::build_from_b_move(self.best_move);
        ts_best_move::TimestampedBestMove::build(best_move, self.timestamp, self.engine_id.clone())
    }
}
