use crate::ui::notation::long_notation;
use crate::entity::engine::component::engine_logic as logic;

#[derive(Debug, Clone)]
pub struct TimestampedBestMove {
    best_move: long_notation::LongAlgebricNotationMove,
    timestamp: chrono::DateTime<chrono::Utc>, // date of best_move initialization
    engine_id: logic::EngineId,               // which engine has found the best move
}
impl TimestampedBestMove {
    fn build(
        best_move: long_notation::LongAlgebricNotationMove,
        timestamp: chrono::DateTime<chrono::Utc>,
        engine_id: logic::EngineId,
    ) -> Self {
        Self {
            best_move,
            timestamp,
            engine_id,
        }
    }
    pub fn best_move(&self) -> long_notation::LongAlgebricNotationMove {
        self.best_move
    }
    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }
    pub fn origin(&self) -> logic::EngineId {
        self.engine_id.clone()
    }
    fn is_more_recent_best_move_than(&self, timestamped_best_move: &TimestampedBestMove) -> bool {
        self.timestamp > timestamped_best_move.timestamp
    }
}