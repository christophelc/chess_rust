use std::time;

use crate::entity::game::component::{game_state, square};

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
    remaining_time_white_black_ms_opt: Option<(u64, u64)>,
    game: &game_state::GameState,
) -> time::Duration {
    let default_duration = time::Duration::from_secs(3600);
    let player_turn = game.bit_position().bit_position_status().player_turn();

    let n_half_moves = game.bit_position().bit_position_status().n_half_moves() as u64;
    let max_time_opt =
        remaining_time_white_black_ms_opt.map(|clock_white_black| match player_turn {
            square::Color::White => time::Duration::from_secs(compute_allocate_time_in_sec(
                clock_white_black.0,
                n_half_moves,
            )),
            square::Color::Black => time::Duration::from_secs(compute_allocate_time_in_sec(
                clock_white_black.1,
                n_half_moves,
            )),
        });
    max_time_opt.unwrap_or(default_duration)
}
