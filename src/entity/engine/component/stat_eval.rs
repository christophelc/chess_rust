#[derive(Default)]
pub struct StatEval {
    n_positions_evaluated: u64,
    n_transposition_hit: u64,
}
impl StatEval {
    pub fn n_positions_evaluated(&self) -> u64 {
        self.n_positions_evaluated
    }
    pub fn n_transposition_hit(&self) -> u64 {
        self.n_transposition_hit
    }
    pub fn inc_n_positions_evaluated(&mut self) -> u64 {
        self.n_positions_evaluated += 1;
        self.n_positions_evaluated
    }
    pub fn inc_n_transposition_hit(&mut self) -> u64 {
        self.n_transposition_hit += 1;
        self.n_transposition_hit
    }
    pub fn reset(&mut self) {
        self.n_positions_evaluated = 0;
    }
}
