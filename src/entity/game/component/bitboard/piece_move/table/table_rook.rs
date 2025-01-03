use crate::entity::game::component::bitboard;

use super::{
    MASK_ROW_1, MASK_ROW_2, MASK_ROW_3, MASK_ROW_4, MASK_ROW_5, MASK_ROW_6, MASK_ROW_7, MASK_ROW_8,
};

const LEFT_MOVES: [u8; 128] = [
    // 0 - 7
    127, 127, 126, 126, 124, 124, 124, 124, // 8 - 15
    120, 120, 120, 120, 120, 120, 120, 120, // 16-23
    112, 112, 112, 112, 112, 112, 112, 112, // 24-31
    112, 112, 112, 112, 112, 112, 112, 112, // 32-39
    96, 96, 96, 96, 96, 96, 96, 96, // 40-47
    96, 96, 96, 96, 96, 96, 96, 96, // 48-55
    96, 96, 96, 96, 96, 96, 96, 96, // 56-63
    96, 96, 96, 96, 96, 96, 96, 96, // 64-71
    64, 64, 64, 64, 64, 64, 64, 64, // 72-79
    64, 64, 64, 64, 64, 64, 64, 64, // 80-87
    64, 64, 64, 64, 64, 64, 64, 64, // 88-95
    64, 64, 64, 64, 64, 64, 64, 64, // 96-103
    64, 64, 64, 64, 64, 64, 64, 64, // 104-111
    64, 64, 64, 64, 64, 64, 64, 64, // 112-119
    64, 64, 64, 64, 64, 64, 64, 64, // 120-127
    64, 64, 64, 64, 64, 64, 64, 64,
];

const RIGHT_MOVES: [u8; 128] = [
    // 0 - 7
    127, 1, 3, 1, 7, 1, 3, 1, // 8 - 15
    15, 1, 3, 1, 7, 1, 3, 1, // 16-23
    31, 1, 3, 1, 7, 1, 3, 1, // 24-31
    15, 1, 3, 1, 7, 1, 3, 1, // 32-39
    63, 1, 3, 1, 7, 1, 3, 1, // 40-47
    15, 1, 3, 1, 7, 1, 3, 1, // 48-55
    31, 1, 3, 1, 7, 1, 3, 1, // 56-63
    15, 1, 3, 1, 7, 1, 3, 1, // 64-71
    95, 1, 3, 1, 7, 1, 3, 1, // 72-79
    15, 1, 3, 1, 7, 1, 3, 1, // 80-87
    31, 1, 3, 1, 7, 1, 3, 1, // 88-95
    15, 1, 3, 1, 7, 1, 3, 1, // 96-103
    63, 1, 3, 1, 7, 1, 3, 1, // 104-111
    15, 1, 3, 1, 7, 1, 3, 1, // 112-119
    31, 1, 3, 1, 7, 1, 3, 1, // 120-127
    15, 1, 3, 1, 7, 1, 3, 1,
];

const MASK_ROWS: [(u64, u64); 8] = [
    (
        MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        0,
    ),
    (
        MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        MASK_ROW_1,
    ),
    (
        MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        MASK_ROW_1 | MASK_ROW_2,
    ),
    (
        MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3,
    ),
    (
        MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4,
    ),
    (
        MASK_ROW_7 | MASK_ROW_8,
        MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5,
    ),
    (
        MASK_ROW_8,
        MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6,
    ),
    (
        0,
        MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7,
    ),
];

pub fn table_rook_h(col: u8, blockers: u8) -> u8 {
    match col {
        0 => RIGHT_MOVES[((blockers & 254) as usize) >> 1] << 1,
        1 => {
            LEFT_MOVES[(blockers & 1) as usize] & 1
                | RIGHT_MOVES[((blockers & 252) as usize) >> 2] << 2
        }
        2 => {
            LEFT_MOVES[(blockers & 3) as usize] & 3
                | RIGHT_MOVES[((blockers & 248) as usize) >> 3] << 3
        }
        3 => {
            LEFT_MOVES[(blockers & 7) as usize] & 7
                | RIGHT_MOVES[((blockers & 240) as usize) >> 4] << 4
        }
        4 => {
            LEFT_MOVES[(blockers & 15) as usize] & 15
                | RIGHT_MOVES[((blockers & 224) as usize) >> 5] << 5
        }
        5 => {
            LEFT_MOVES[(blockers & 31) as usize] & 31
                | RIGHT_MOVES[((blockers & 192) as usize) >> 6] << 6
        }
        6 => {
            LEFT_MOVES[(blockers & 63) as usize] & 63
                | RIGHT_MOVES[((blockers & 128) as usize) >> 7] << 7
        }
        7 => LEFT_MOVES[(blockers & 127) as usize],
        _ => panic!("For rook, col {col} should not happen."),
    }
}

pub fn table_rook_v(index: u8, blockers: u64, mask_col: u64) -> u64 {
    let row = index / 8;
    let (mask_up, mask_down) = MASK_ROWS[row as usize];
    let moves_up: u64 = if mask_up == 0 {
        0
    } else {
        let mask_up = mask_up & mask_col;
        let v_up = blockers & mask_up;
        let number_of_1_from_left_index = (bitboard::pos2index(v_up) - index) / 8;
        mask_up & super::get_mask_row_up(row, number_of_1_from_left_index)
    };
    let moves_down: u64 = if mask_down == 0 {
        0
    } else {
        let mask_down = mask_down & mask_col;
        let v_down = blockers & mask_down;
        let number_of_1_from_right_index = (v_down.leading_zeros() as u8 - (63 - index)) / 8;
        mask_down & super::get_mask_row_down(row, number_of_1_from_right_index)
    };
    moves_up | moves_down
}
