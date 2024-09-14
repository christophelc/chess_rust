use super::{
    MASK_ROW_0, MASK_ROW_1, MASK_ROW_2, MASK_ROW_3, MASK_ROW_4, MASK_ROW_5, MASK_ROW_6, MASK_ROW_7,
};
use crate::board::bitboard;
use core::num;

#[rustfmt::skip]
const MASK_BISHOP_DIAG1: [(u64, u64); 64] = [
    (0, 0), 
    (1u64 << 8, 0), 
    (1u64 << 9 | 1u64 << 16, 0), 
    (1u64 << 10 | 1u64 << 17 | 1u64 << 24, 0), 
    (1u64 << 11 | 1u64 << 18 | 1u64 << 25 | 1u64 << 32, 0), 
    (1u64 << 12 | 1u64 << 19 | 1u64 << 26 | 1u64 << 33 | 1u64 << 40, 0), 
    (1u64 << 13 | 1u64 << 20 | 1u64 << 27 | 1u64 << 34 | 1u64 << 41 | 1u64 << 48, 0), 
    (1u64 << 14 | 1u64 << 21 | 1u64 << 28 | 1u64 << 35 | 1u64 << 42 | 1u64 << 49 | 1u64 << 56, 0), 
    (0, 1u64 << 1), 
    (1u64 << 16, 1u64 << 2), 
    (1u64 << 17 | 1u64 << 24, 1u64 << 3), 
    (1u64 << 18 | 1u64 << 25 | 1u64 << 32, 1u64 << 4), 
    (1u64 << 19 | 1u64 << 26 | 1u64 << 33 | 1u64 << 40, 1u64 << 5), 
    (1u64 << 20 | 1u64 << 27 | 1u64 << 34 | 1u64 << 41 | 1u64 << 48, 1u64 << 6), 
    (1u64 << 21 | 1u64 << 28 | 1u64 << 35 | 1u64 << 42 | 1u64 << 49 | 1u64 << 56, 1u64 << 7), 
    (1u64 << 22 | 1u64 << 29 | 1u64 << 36 | 1u64 << 43 | 1u64 << 50 | 1u64 << 57, 0), 
    (0, 1u64 << 2 | 1u64 << 9), 
    (1u64 << 24, 1u64 << 3 | 1u64 << 10), 
    (1u64 << 25 | 1u64 << 32, 1u64 << 4 | 1u64 << 11), 
    (1u64 << 26 | 1u64 << 33 | 1u64 << 40, 1u64 << 5 | 1u64 << 12), 
    (1u64 << 27 | 1u64 << 34 | 1u64 << 41 | 1u64 << 48, 1u64 << 6 | 1u64 << 13), 
    (1u64 << 28 | 1u64 << 35 | 1u64 << 42 | 1u64 << 49 | 1u64 << 56, 1u64 << 7 | 1u64 << 14), 
    (1u64 << 29 | 1u64 << 36 | 1u64 << 43 | 1u64 << 50 | 1u64 << 57, 1u64 << 15), 
    (1u64 << 30 | 1u64 << 37 | 1u64 << 44 | 1u64 << 51 | 1u64 << 58, 0), 
    (0, 1u64 << 3 | 1u64 << 10 | 1u64 << 17), 
    (1u64 << 32, 1u64 << 4 | 1u64 << 11 | 1u64 << 18), 
    (1u64 << 33 | 1u64 << 40, 1u64 << 5 | 1u64 << 12 | 1u64 << 19), 
    (1u64 << 34 | 1u64 << 41 | 1u64 << 48, 1u64 << 6 | 1u64 << 13 | 1u64 << 20), 
    (1u64 << 35 | 1u64 << 42 | 1u64 << 49 | 1u64 << 56, 1u64 << 7 | 1u64 << 14 | 1u64 << 21), 
    (1u64 << 36 | 1u64 << 43 | 1u64 << 50 | 1u64 << 57, 1u64 << 15 | 1u64 << 22), 
    (1u64 << 37 | 1u64 << 44 | 1u64 << 51 | 1u64 << 58, 1u64 << 23), 
    (1u64 << 38 | 1u64 << 45 | 1u64 << 52 | 1u64 << 59, 0), 
    (0, 1u64 << 4 | 1u64 << 11 | 1u64 << 18 | 1u64 << 25), 
    (1u64 << 40, 1u64 << 5 | 1u64 << 12 | 1u64 << 19 | 1u64 << 26), 
    (1u64 << 41 | 1u64 << 48, 1u64 << 6 | 1u64 << 13 | 1u64 << 20 | 1u64 << 27), 
    (1u64 << 42 | 1u64 << 49 | 1u64 << 56, 1u64 << 7 | 1u64 << 14 | 1u64 << 21 | 1u64 << 28), 
    (1u64 << 43 | 1u64 << 50 | 1u64 << 57, 1u64 << 15 | 1u64 << 22 | 1u64 << 29), 
    (1u64 << 44 | 1u64 << 51 | 1u64 << 58, 1u64 << 23 | 1u64 << 30), 
    (1u64 << 45 | 1u64 << 52 | 1u64 << 59, 1u64 << 31), 
    (1u64 << 46 | 1u64 << 53 | 1u64 << 60, 0), 
    (0, 1u64 << 5 | 1u64 << 12 | 1u64 << 19 | 1u64 << 26 | 1u64 << 33), 
    (1u64 << 48, 1u64 << 6 | 1u64 << 13 | 1u64 << 20 | 1u64 << 27 | 1u64 << 34), 
    (1u64 << 49 | 1u64 << 56, 1u64 << 7 | 1u64 << 14 | 1u64 << 21 | 1u64 << 28 | 1u64 << 35), 
    (1u64 << 50 | 1u64 << 57, 1u64 << 15 | 1u64 << 22 | 1u64 << 29 | 1u64 << 36), 
    (1u64 << 51 | 1u64 << 58, 1u64 << 23 | 1u64 << 30 | 1u64 << 37), 
    (1u64 << 52 | 1u64 << 59, 1u64 << 31 | 1u64 << 38), 
    (1u64 << 53 | 1u64 << 60, 1u64 << 39), 
    (1u64 << 54 | 1u64 << 61, 0), 
    (0, 1u64 << 6 | 1u64 << 13 | 1u64 << 20 | 1u64 << 27 | 1u64 << 34 | 1u64 << 41), 
    (1u64 << 56, 1u64 << 7 | 1u64 << 14 | 1u64 << 21 | 1u64 << 28 | 1u64 << 35 | 1u64 << 42), 
    (1u64 << 57, 1u64 << 15 | 1u64 << 22 | 1u64 << 29 | 1u64 << 36 | 1u64 << 43), 
    (1u64 << 58, 1u64 << 23 | 1u64 << 30 | 1u64 << 37 | 1u64 << 44), 
    (1u64 << 59, 1u64 << 31 | 1u64 << 38 | 1u64 << 45), 
    (1u64 << 60, 1u64 << 39 | 1u64 << 46), 
    (1u64 << 61, 1u64 << 47), 
    (1u64 << 62, 0), 
    (0, 1u64 << 7 | 1u64 << 14 | 1u64 << 21 | 1u64 << 28 | 1u64 << 35 | 1u64 << 42 | 1u64 << 49), 
    (0, 1u64 << 15 | 1u64 << 22 | 1u64 << 29 | 1u64 << 36 | 1u64 << 43 | 1u64 << 50), 
    (0, 1u64 << 23 | 1u64 << 30 | 1u64 << 37 | 1u64 << 44 | 1u64 << 51), 
    (0, 1u64 << 31 | 1u64 << 38 | 1u64 << 45 | 1u64 << 52), 
    (0, 1u64 << 39 | 1u64 << 46 | 1u64 << 53), 
    (0, 1u64 << 47 | 1u64 << 54), 
    (0, 1u64 << 55), 
    (0, 0), 
];

#[rustfmt::skip]
const MASK_BISHOP_DIAG2: [(u64, u64); 64] = [
    (0, 1u64 << 9 | 1u64 << 18 | 1u64 << 27 | 1u64 << 36 | 1u64 << 45 | 1u64 << 54 | 1u64 << 63), 
    (0, 1u64 << 10 | 1u64 << 19 | 1u64 << 28 | 1u64 << 37 | 1u64 << 46 | 1u64 << 55), 
    (0, 1u64 << 11 | 1u64 << 20 | 1u64 << 29 | 1u64 << 38 | 1u64 << 47), 
    (0, 1u64 << 12 | 1u64 << 21 | 1u64 << 30 | 1u64 << 39), 
    (0, 1u64 << 13 | 1u64 << 22 | 1u64 << 31), 
    (0, 1u64 << 14 | 1u64 << 23), 
    (0, 1u64 << 15), 
    (0, 0), 
    (0, 1u64 << 17 | 1u64 << 26 | 1u64 << 35 | 1u64 << 44 | 1u64 << 53 | 1u64 << 62), 
    (1u64 << 0, 1u64 << 18 | 1u64 << 27 | 1u64 << 36 | 1u64 << 45 | 1u64 << 54 | 1u64 << 63), 
    (1u64 << 1, 1u64 << 19 | 1u64 << 28 | 1u64 << 37 | 1u64 << 46 | 1u64 << 55), 
    (1u64 << 2, 1u64 << 20 | 1u64 << 29 | 1u64 << 38 | 1u64 << 47), 
    (1u64 << 3, 1u64 << 21 | 1u64 << 30 | 1u64 << 39), 
    (1u64 << 4, 1u64 << 22 | 1u64 << 31), 
    (1u64 << 5, 1u64 << 23), 
    (1u64 << 6, 0), 
    (0, 1u64 << 25 | 1u64 << 34 | 1u64 << 43 | 1u64 << 52 | 1u64 << 61), 
    (1u64 << 8, 1u64 << 26 | 1u64 << 35 | 1u64 << 44 | 1u64 << 53 | 1u64 << 62), 
    (1u64 << 0 | 1u64 << 9, 1u64 << 27 | 1u64 << 36 | 1u64 << 45 | 1u64 << 54 | 1u64 << 63), 
    (1u64 << 1 | 1u64 << 10, 1u64 << 28 | 1u64 << 37 | 1u64 << 46 | 1u64 << 55), 
    (1u64 << 2 | 1u64 << 11, 1u64 << 29 | 1u64 << 38 | 1u64 << 47), 
    (1u64 << 3 | 1u64 << 12, 1u64 << 30 | 1u64 << 39), 
    (1u64 << 4 | 1u64 << 13, 1u64 << 31), 
    (1u64 << 5 | 1u64 << 14, 0), 
    (0, 1u64 << 33 | 1u64 << 42 | 1u64 << 51 | 1u64 << 60), 
    (1u64 << 16, 1u64 << 34 | 1u64 << 43 | 1u64 << 52 | 1u64 << 61), 
    (1u64 << 8 | 1u64 << 17, 1u64 << 35 | 1u64 << 44 | 1u64 << 53 | 1u64 << 62), 
    (1u64 << 0 | 1u64 << 9 | 1u64 << 18, 1u64 << 36 | 1u64 << 45 | 1u64 << 54 | 1u64 << 63), 
    (1u64 << 1 | 1u64 << 10 | 1u64 << 19, 1u64 << 37 | 1u64 << 46 | 1u64 << 55), 
    (1u64 << 2 | 1u64 << 11 | 1u64 << 20, 1u64 << 38 | 1u64 << 47), 
    (1u64 << 3 | 1u64 << 12 | 1u64 << 21, 1u64 << 39), 
    (1u64 << 4 | 1u64 << 13 | 1u64 << 22, 0), 
    (0, 1u64 << 41 | 1u64 << 50 | 1u64 << 59), 
    (1u64 << 24, 1u64 << 42 | 1u64 << 51 | 1u64 << 60), 
    (1u64 << 16 | 1u64 << 25, 1u64 << 43 | 1u64 << 52 | 1u64 << 61), 
    (1u64 << 8 | 1u64 << 17 | 1u64 << 26, 1u64 << 44 | 1u64 << 53 | 1u64 << 62), 
    (1u64 << 0 | 1u64 << 9 | 1u64 << 18 | 1u64 << 27, 1u64 << 45 | 1u64 << 54 | 1u64 << 63), 
    (1u64 << 1 | 1u64 << 10 | 1u64 << 19 | 1u64 << 28, 1u64 << 46 | 1u64 << 55), 
    (1u64 << 2 | 1u64 << 11 | 1u64 << 20 | 1u64 << 29, 1u64 << 47), 
    (1u64 << 3 | 1u64 << 12 | 1u64 << 21 | 1u64 << 30, 0), 
    (0, 1u64 << 49 | 1u64 << 58), 
    (1u64 << 32, 1u64 << 50 | 1u64 << 59), 
    (1u64 << 24 | 1u64 << 33, 1u64 << 51 | 1u64 << 60), 
    (1u64 << 16 | 1u64 << 25 | 1u64 << 34, 1u64 << 52 | 1u64 << 61), 
    (1u64 << 8 | 1u64 << 17 | 1u64 << 26 | 1u64 << 35, 1u64 << 53 | 1u64 << 62), 
    (1u64 << 0 | 1u64 << 9 | 1u64 << 18 | 1u64 << 27 | 1u64 << 36, 1u64 << 54 | 1u64 << 63), 
    (1u64 << 1 | 1u64 << 10 | 1u64 << 19 | 1u64 << 28 | 1u64 << 37, 1u64 << 55), 
    (1u64 << 2 | 1u64 << 11 | 1u64 << 20 | 1u64 << 29 | 1u64 << 38, 0), 
    (0, 1u64 << 57), 
    (1u64 << 40, 1u64 << 58), 
    (1u64 << 32 | 1u64 << 41, 1u64 << 59), 
    (1u64 << 24 | 1u64 << 33 | 1u64 << 42, 1u64 << 60), 
    (1u64 << 16 | 1u64 << 25 | 1u64 << 34 | 1u64 << 43, 1u64 << 61), 
    (1u64 << 8 | 1u64 << 17 | 1u64 << 26 | 1u64 << 35 | 1u64 << 44, 1u64 << 62), 
    (1u64 << 0 | 1u64 << 9 | 1u64 << 18 | 1u64 << 27 | 1u64 << 36 | 1u64 << 45, 1u64 << 63), 
    (1u64 << 1 | 1u64 << 10 | 1u64 << 19 | 1u64 << 28 | 1u64 << 37 | 1u64 << 46, 0), 
    (0, 0), 
    (1u64 << 48, 0), 
    (1u64 << 40 | 1u64 << 49, 0), 
    (1u64 << 32 | 1u64 << 41 | 1u64 << 50, 0), 
    (1u64 << 24 | 1u64 << 33 | 1u64 << 42 | 1u64 << 51, 0), 
    (1u64 << 16 | 1u64 << 25 | 1u64 << 34 | 1u64 << 43 | 1u64 << 52, 0), 
    (1u64 << 8 | 1u64 << 17 | 1u64 << 26 | 1u64 << 35 | 1u64 << 44 | 1u64 << 53, 0), 
    (1u64 << 0 | 1u64 << 9 | 1u64 << 18 | 1u64 << 27 | 1u64 << 36 | 1u64 << 45 | 1u64 << 54, 0), 
];

pub fn bishop_moves(index: u8, blockers: u64) -> u64 {
    let row = index / 8;
    let (mask_l, mask_r) = MASK_BISHOP_DIAG1[index as usize];
    let moves_diag_1 = bishop_moves_diag(row, index, blockers, mask_l, mask_r, 7);
    let (mask_l, mask_r) = MASK_BISHOP_DIAG2[index as usize];
    let moves_diag_2 = bishop_moves_diag(row, index, blockers, mask_r, mask_l, 9);
    moves_diag_1 | moves_diag_2
}
pub fn bishop_moves_diag(
    row: u8,
    index: u8,
    blockers: u64,
    mask_up: u64,
    mask_down: u64,
    step: u8,
) -> u64 {
    let moves_up = if mask_up == 0 {
        0
    } else {
        let v_up = mask_up & blockers;
        let number_of_1_from_left_index = (bitboard::pos2index(v_up) - index) / step;
        mask_up & super::get_mask_row_up(row, number_of_1_from_left_index)
    };
    let moves_down = if mask_down == 0 {
        0
    } else {
        let v_down = mask_down & blockers;
        let number_of_1_from_right_index = (v_down.leading_zeros() as u8 - (63 - index)) / step;
        mask_down & super::get_mask_row_down(row, number_of_1_from_right_index)
    };
    moves_up | moves_down
}

mod tests {
    use super::*;
    use std::cmp;

    fn compute_diag1(index: u8) -> u64 {
        let row = index / 8;
        let col = index % 8;
        let distance_min = cmp::min(col, 7 - row);
        let (x0, y0) = (col - distance_min, row + distance_min);
        let mut diag: u64 = 0;
        for i in 0..=cmp::min(7 - x0, y0) {
            diag |= 1u64 << x0 + i + (y0 - i) * 8;
        }
        diag
    }
    fn compute_diag2(index: u8) -> u64 {
        let row = index / 8;
        let col = index % 8;
        let distance_min = cmp::min(col, row);
        let (x0, y0) = (col - distance_min, row - distance_min);
        let mut diag: u64 = 0;
        for i in 0..=cmp::min(7 - x0, 7 - y0) {
            diag |= 1u64 << x0 + i + (y0 + i) * 8;
        }
        diag
    }

    #[test]
    #[ignore]
    fn show_diag1() {
        println!("[");
        for index in 0..64 {
            let mut diags = compute_diag1(index);
            let mut v_left = Vec::new();
            let mut v_right = Vec::new();
            while diags != 0 {
                let lsb = bitboard::pos2index(diags);
                let s = "1u64 << ".to_string() + &lsb.to_string();
                if lsb != index {
                    if lsb / 8 > index / 8 && lsb - lsb % 8 > index - index % 8 {
                        v_left.push(s);
                    } else {
                        v_right.push(s);
                    }
                }
                diags &= diags - 1; // Remove lsb
            }
            println!(
                "({}, {}), ",
                if v_left.is_empty() {
                    "0".to_string()
                } else {
                    v_left.join(" | ")
                },
                if v_right.is_empty() {
                    "0".to_string()
                } else {
                    v_right.join(" | ")
                }
            );
        }
        println!("];");
    }

    #[test]
    #[ignore]
    fn show_diag2() {
        println!("[");
        for index in 0..64 {
            let mut diags = compute_diag2(index);
            let mut v_left = Vec::new();
            let mut v_right = Vec::new();
            while diags != 0 {
                let lsb = diags.trailing_zeros() as u8;
                let s = "1u64 << ".to_string() + &lsb.to_string();
                if lsb != index {
                    if lsb / 8 < index / 8 && lsb - lsb % 8 < index - index % 8 {
                        v_left.push(s);
                    } else {
                        v_right.push(s);
                    }
                }
                diags &= diags - 1; // Remove lsb
            }
            println!(
                "({}, {}), ",
                if v_left.is_empty() {
                    "0".to_string()
                } else {
                    v_left.join(" | ")
                },
                if v_right.is_empty() {
                    "0".to_string()
                } else {
                    v_right.join(" | ")
                }
            );
        }
        println!("];");
    }

    #[test]
    //#[ignore]
    fn test_bishop_move() {
        let index: u8 = 41;
        //let index: u8 = 19;
        let blockers: u64 = 1u64 << 20;
        bishop_moves(index, blockers);
    }

    ////////////////////////////////////////////////////////
    /// Bishop moves
    ////////////////////////////////////////////////////////
    #[test]
    fn test_table_bishop_no_blockers() {
        let index = 27; // This corresponds to row 3, column 3 on an 8x8 board.
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected = (1u64 << 0)
            | (1u64 << 9)
            | (1u64 << 18)
            | (1u64 << 36)
            | (1u64 << 45)
            | (1u64 << 54)
            | (1u64 << 63)
            | (1u64 << 6)
            | (1u64 << 13)
            | (1u64 << 20)
            | (1u64 << 34)
            | (1u64 << 41)
            | (1u64 << 48);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_edge_of_board() {
        let index = 0;
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected = (1u64 << 9)
            | (1u64 << 18)
            | (1u64 << 27)
            | (1u64 << 36)
            | (1u64 << 45)
            | (1u64 << 54)
            | (1u64 << 63);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_corner_case() {
        let index = 63;
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected = (1u64 << 54)
            | (1u64 << 45)
            | (1u64 << 36)
            | (1u64 << 27)
            | (1u64 << 18)
            | (1u64 << 9)
            | (1u64 << 0);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_central_position() {
        let index = 28;
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected = (1u64 << 19)
            | (1u64 << 10)
            | (1u64 << 1)
            | (1u64 << 37)
            | (1u64 << 46)
            | (1u64 << 55)
            | (1u64 << 56
                | 1u64 << 49
                | 1u64 << 42
                | 1u64 << 35
                | 1u64 << 21
                | 1u64 << 14
                | 1u64 << 7);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_single_blocker() {
        let index = 36;
        let blockers = 1u64 << 27;
        let moves = bishop_moves(index, blockers);
        let expected = (1u64 << 45)
            | (1u64 << 54)
            | (1u64 << 63)
            | (1u64 << 27)
            | (1u64 << 15)
            | (1u64 << 22)
            | (1u64 << 29)
            | (1u64 << 43)
            | (1u64 << 50 | 1u64 << 57);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_multiple_blockers() {
        let index = 36;
        let blockers = (1u64 << 27) | (1u64 << 45);
        let moves = bishop_moves(index, blockers);
        let expected = (1u64 << 27)
            | (1u64 << 45)
            | (1u64 << 15)
            | (1u64 << 22 | 1u64 << 29 | 1u64 << 43 | 1u64 << 50 | 1u64 << 57);
        assert_eq!(moves, expected);
    }
}
