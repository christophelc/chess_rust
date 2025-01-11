use std::time;

use crate::entity::game::component::game_state;

const AVG_LENGTH_GAME: u64 = 220;

fn compute_allocate_time_in_sec(time_remaining_ms: u64, n_half_moves: u64) -> u64 {
    // very basic allocation
    let allocation_in_ms = if n_half_moves < 10 {
        5000
    } else if n_half_moves > AVG_LENGTH_GAME {
        time_remaining_ms / 20
    } else {
        2 * time_remaining_ms / (AVG_LENGTH_GAME - n_half_moves)
    };
    if allocation_in_ms > 1000 {
        allocation_in_ms / 1000
    } else {
        1
    }
}

pub fn estimate_time_allocation(
    remaining_time_ms_opt: Option<u64>,
    game: &game_state::GameState,
) -> Option<time::Duration> {
    let n_half_moves = game.bit_position().bit_position_status().n_half_moves() as u64;
    let max_time_opt = remaining_time_ms_opt
        .map(|clock| time::Duration::from_secs(compute_allocate_time_in_sec(clock, n_half_moves)));
    max_time_opt
}
