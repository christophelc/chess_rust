pub mod table_bishop;
pub mod table_king;
pub mod table_knight;
pub mod table_rook;

pub const MASK_ROW_1: u64 = 255;
const MASK_ROW_2: u64 = MASK_ROW_1 << 8;
const MASK_ROW_3: u64 = MASK_ROW_1 << 16;
pub const MASK_ROW_4: u64 = MASK_ROW_1 << 24;
pub const MASK_ROW_5: u64 = MASK_ROW_1 << 32;
const MASK_ROW_6: u64 = MASK_ROW_1 << 40;
const MASK_ROW_7: u64 = MASK_ROW_1 << 48;
pub const MASK_ROW_8: u64 = MASK_ROW_1 << 56;

pub const MASK_COL_A: u64 = 0x0101010101010101;
pub const MASK_COL_D: u64 = MASK_COL_A << 3;
pub const MASK_COL_E: u64 = MASK_COL_A << 4;
pub const MASK_COL_H: u64 = MASK_COL_A << 7;

fn get_mask_row_up(row: u8, n: u8) -> u64 {
    let n = if n == 0 { 1 } else { n };
    match (row, n) {
        (6, _) => MASK_ROW_8,
        (5, 1) => MASK_ROW_7,
        (5, _) => MASK_ROW_7 | MASK_ROW_8,
        (4, 1) => MASK_ROW_6,
        (4, 2) => MASK_ROW_6 | MASK_ROW_7,
        (4, _) => MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        (3, 1) => MASK_ROW_5,
        (3, 2) => MASK_ROW_5 | MASK_ROW_6,
        (3, 3) => MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7,
        (3, _) => MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        (2, 1) => MASK_ROW_4,
        (2, 2) => MASK_ROW_4 | MASK_ROW_5,
        (2, 3) => MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6,
        (2, 4) => MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7,
        (2, _) => MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        (1, 1) => MASK_ROW_3,
        (1, 2) => MASK_ROW_3 | MASK_ROW_4,
        (1, 3) => MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5,
        (1, 4) => MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6,
        (1, 5) => MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7,
        (1, _) => MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8,
        (0, 1) => MASK_ROW_2,
        (0, 2) => MASK_ROW_2 | MASK_ROW_3,
        (0, 3) => MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4,
        (0, 4) => MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5,
        (0, 5) => MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6,
        (0, 6) => MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7,
        (0, _) => {
            MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5 | MASK_ROW_6 | MASK_ROW_7 | MASK_ROW_8
        }
        _ => panic!("mask_up invalid case: bishop_row={} n={}", row, n),
    }
}
fn get_mask_row_down(bishop_row: u8, n: u8) -> u64 {
    let n = if n == 0 { 1 } else { n };
    match (bishop_row, n) {
        (1, _) => MASK_ROW_1,
        (2, 1) => MASK_ROW_2,
        (2, _) => MASK_ROW_2 | MASK_ROW_1,
        (3, 1) => MASK_ROW_3,
        (3, 2) => MASK_ROW_3 | MASK_ROW_2,
        (3, _) => MASK_ROW_3 | MASK_ROW_2 | MASK_ROW_1,
        (4, 1) => MASK_ROW_4,
        (4, 2) => MASK_ROW_4 | MASK_ROW_3,
        (4, 3) => MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2,
        (4, _) => MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2 | MASK_ROW_1,
        (5, 1) => MASK_ROW_5,
        (5, 2) => MASK_ROW_5 | MASK_ROW_4,
        (5, 3) => MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3,
        (5, 4) => MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2,
        (5, _) => MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2 | MASK_ROW_1,
        (6, 1) => MASK_ROW_6,
        (6, 2) => MASK_ROW_6 | MASK_ROW_5,
        (6, 3) => MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4,
        (6, 4) => MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3,
        (6, 5) => MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2,
        (6, _) => MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2 | MASK_ROW_1,
        (7, 1) => MASK_ROW_7,
        (7, 2) => MASK_ROW_7 | MASK_ROW_6,
        (7, 3) => MASK_ROW_7 | MASK_ROW_6 | MASK_ROW_5,
        (7, 4) => MASK_ROW_7 | MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4,
        (7, 5) => MASK_ROW_7 | MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3,
        (7, 6) => MASK_ROW_7 | MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2,
        (7, _) => {
            MASK_ROW_7 | MASK_ROW_6 | MASK_ROW_5 | MASK_ROW_4 | MASK_ROW_3 | MASK_ROW_2 | MASK_ROW_1
        }
        _ => panic!("mask_down invalid case: bishop_row={} n={}", bishop_row, n),
    }
}
