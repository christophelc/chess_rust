#[derive(Default)]
pub struct StatEval {
    n_positions_evaluated: u64,
    n_transposition_hit: u64,
    n_check: u64,
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
    pub fn reset_n_positions_evaluated(&mut self) {
        self.n_positions_evaluated = 0;
    }
    pub fn inc_n_check(&mut self, n_check: u64) -> u64 {
        self.n_check += n_check;
        self.n_check
    }
}
