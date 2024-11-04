pub const SEND_STAT_EVERY_N_POSITION_EVALUATED: u64 = 5000;

#[derive(Debug, Clone)]
pub struct StatData {
    n_positions_evaluated: u64,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: Option<chrono::DateTime<chrono::Utc>>,
}
impl StatData {
    pub fn new(n_positions_evaluated: u64) -> Self {
        Self {
            n_positions_evaluated,
            start_time: chrono::Utc::now(),
            end_time: None,
        }
    }
    pub fn n_positions_evaluated(&self) -> u64 {
        self.n_positions_evaluated
    }
    pub fn start_time(&self) -> chrono::DateTime<chrono::Utc> {
        self.start_time
    }
    pub fn end_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.end_time
    }
    pub fn close(&mut self) {
        assert!(self.end_time.is_none());
        self.end_time = Some(chrono::Utc::now());
    }
    pub fn inc(&mut self, n_inc: u64) {
        self.n_positions_evaluated += n_inc;
    }
    pub fn set(&mut self, n: u64) {
        self.n_positions_evaluated = n;
    }
}
