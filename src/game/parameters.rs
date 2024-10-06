use crate::uci::notation::LongAlgebricNotationMove;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parameters {
    opt_depth: Option<u32>,
    opt_time_per_move_in_ms: Option<u32>,
    opt_wtime: Option<u64>,
    opt_btime: Option<u64>,
    opt_wtime_inc: Option<u64>,
    opt_btime_inc: Option<u64>,
    search_moves: Vec<LongAlgebricNotationMove>,
}

impl Parameters {
    #[cfg(test)]
    pub fn new(
        opt_depth: Option<u32>,
        opt_time_per_move_in_ms: Option<u32>,
        opt_wtime: Option<u64>,
        opt_btime: Option<u64>,
        opt_wtime_inc: Option<u64>,
        opt_btime_inc: Option<u64>,
        search_moves: Vec<LongAlgebricNotationMove>,
    ) -> Self {
        Parameters {
            opt_depth,
            opt_time_per_move_in_ms,
            opt_wtime,
            opt_btime,
            opt_wtime_inc,
            opt_btime_inc,
            search_moves,
        }
    }
    pub fn set_depth(&mut self, depth: u32) {
        self.opt_depth = Some(depth);
    }
    pub fn set_depth_infinite(&mut self) {
        self.opt_depth = None;
    }
    pub fn set_time_per_move_in_ms(&mut self, time_per_move_in_ms: u32) {
        self.opt_time_per_move_in_ms = Some(time_per_move_in_ms);
    }
    pub fn set_wtime(&mut self, wtime: u64) {
        self.opt_wtime = Some(wtime);
    }
    pub fn set_btime(&mut self, btime: u64) {
        self.opt_btime = Some(btime);
    }
    pub fn set_wtime_inc(&mut self, wtime_inc: u64) {
        self.opt_wtime_inc = Some(wtime_inc);
    }
    pub fn set_btime_inc(&mut self, btime_inc: u64) {
        self.opt_btime_inc = Some(btime_inc);
    }
    pub fn set_search_moves(&mut self, search_moves: Vec<LongAlgebricNotationMove>) {
        self.search_moves = search_moves;
    }
}
