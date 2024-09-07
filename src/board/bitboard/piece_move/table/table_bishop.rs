use super::{
    MASK_ROW_0, MASK_ROW_1, MASK_ROW_2, MASK_ROW_3, MASK_ROW_4, MASK_ROW_5, MASK_ROW_6, MASK_ROW_7,
};
use crate::board::bitboard;
use core::num;

#[rustfmt::skip]
const MASK_BISHOP_DIAG1: [(u64, u64); 64] = [
    (0, 0), 
    (1 << 8, 0), 
    (1 << 9 | 1 << 16, 0), 
    (1 << 10 | 1 << 17 | 1 << 24, 0), 
    (1 << 11 | 1 << 18 | 1 << 25 | 1 << 32, 0), 
    (1 << 12 | 1 << 19 | 1 << 26 | 1 << 33 | 1 << 40, 0), 
    (1 << 13 | 1 << 20 | 1 << 27 | 1 << 34 | 1 << 41 | 1 << 48, 0), 
    (1 << 14 | 1 << 21 | 1 << 28 | 1 << 35 | 1 << 42 | 1 << 49 | 1 << 56, 0), 
    (0, 1 << 1), 
    (1 << 16, 1 << 2), 
    (1 << 17 | 1 << 24, 1 << 3), 
    (1 << 18 | 1 << 25 | 1 << 32, 1 << 4), 
    (1 << 19 | 1 << 26 | 1 << 33 | 1 << 40, 1 << 5), 
    (1 << 20 | 1 << 27 | 1 << 34 | 1 << 41 | 1 << 48, 1 << 6), 
    (1 << 21 | 1 << 28 | 1 << 35 | 1 << 42 | 1 << 49 | 1 << 56, 1 << 7), 
    (1 << 22 | 1 << 29 | 1 << 36 | 1 << 43 | 1 << 50 | 1 << 57, 0), 
    (0, 1 << 2 | 1 << 9), 
    (1 << 24, 1 << 3 | 1 << 10), 
    (1 << 25 | 1 << 32, 1 << 4 | 1 << 11), 
    (1 << 26 | 1 << 33 | 1 << 40, 1 << 5 | 1 << 12), 
    (1 << 27 | 1 << 34 | 1 << 41 | 1 << 48, 1 << 6 | 1 << 13), 
    (1 << 28 | 1 << 35 | 1 << 42 | 1 << 49 | 1 << 56, 1 << 7 | 1 << 14), 
    (1 << 29 | 1 << 36 | 1 << 43 | 1 << 50 | 1 << 57, 1 << 15), 
    (1 << 30 | 1 << 37 | 1 << 44 | 1 << 51 | 1 << 58, 0), 
    (0, 1 << 3 | 1 << 10 | 1 << 17), 
    (1 << 32, 1 << 4 | 1 << 11 | 1 << 18), 
    (1 << 33 | 1 << 40, 1 << 5 | 1 << 12 | 1 << 19), 
    (1 << 34 | 1 << 41 | 1 << 48, 1 << 6 | 1 << 13 | 1 << 20), 
    (1 << 35 | 1 << 42 | 1 << 49 | 1 << 56, 1 << 7 | 1 << 14 | 1 << 21), 
    (1 << 36 | 1 << 43 | 1 << 50 | 1 << 57, 1 << 15 | 1 << 22), 
    (1 << 37 | 1 << 44 | 1 << 51 | 1 << 58, 1 << 23), 
    (1 << 38 | 1 << 45 | 1 << 52 | 1 << 59, 0), 
    (0, 1 << 4 | 1 << 11 | 1 << 18 | 1 << 25), 
    (1 << 40, 1 << 5 | 1 << 12 | 1 << 19 | 1 << 26), 
    (1 << 41 | 1 << 48, 1 << 6 | 1 << 13 | 1 << 20 | 1 << 27), 
    (1 << 42 | 1 << 49 | 1 << 56, 1 << 7 | 1 << 14 | 1 << 21 | 1 << 28), 
    (1 << 43 | 1 << 50 | 1 << 57, 1 << 15 | 1 << 22 | 1 << 29), 
    (1 << 44 | 1 << 51 | 1 << 58, 1 << 23 | 1 << 30), 
    (1 << 45 | 1 << 52 | 1 << 59, 1 << 31), 
    (1 << 46 | 1 << 53 | 1 << 60, 0), 
    (0, 1 << 5 | 1 << 12 | 1 << 19 | 1 << 26 | 1 << 33), 
    (1 << 48, 1 << 6 | 1 << 13 | 1 << 20 | 1 << 27 | 1 << 34), 
    (1 << 49 | 1 << 56, 1 << 7 | 1 << 14 | 1 << 21 | 1 << 28 | 1 << 35), 
    (1 << 50 | 1 << 57, 1 << 15 | 1 << 22 | 1 << 29 | 1 << 36), 
    (1 << 51 | 1 << 58, 1 << 23 | 1 << 30 | 1 << 37), 
    (1 << 52 | 1 << 59, 1 << 31 | 1 << 38), 
    (1 << 53 | 1 << 60, 1 << 39), 
    (1 << 54 | 1 << 61, 0), 
    (0, 1 << 6 | 1 << 13 | 1 << 20 | 1 << 27 | 1 << 34 | 1 << 41), 
    (1 << 56, 1 << 7 | 1 << 14 | 1 << 21 | 1 << 28 | 1 << 35 | 1 << 42), 
    (1 << 57, 1 << 15 | 1 << 22 | 1 << 29 | 1 << 36 | 1 << 43), 
    (1 << 58, 1 << 23 | 1 << 30 | 1 << 37 | 1 << 44), 
    (1 << 59, 1 << 31 | 1 << 38 | 1 << 45), 
    (1 << 60, 1 << 39 | 1 << 46), 
    (1 << 61, 1 << 47), 
    (1 << 62, 0), 
    (0, 1 << 7 | 1 << 14 | 1 << 21 | 1 << 28 | 1 << 35 | 1 << 42 | 1 << 49), 
    (0, 1 << 15 | 1 << 22 | 1 << 29 | 1 << 36 | 1 << 43 | 1 << 50), 
    (0, 1 << 23 | 1 << 30 | 1 << 37 | 1 << 44 | 1 << 51), 
    (0, 1 << 31 | 1 << 38 | 1 << 45 | 1 << 52), 
    (0, 1 << 39 | 1 << 46 | 1 << 53), 
    (0, 1 << 47 | 1 << 54), 
    (0, 1 << 55), 
    (0, 0), 
];

#[rustfmt::skip]
const MASK_BISHOP_DIAG2: [(u64, u64); 64] = [
    (0, 1 << 9 | 1 << 18 | 1 << 27 | 1 << 36 | 1 << 45 | 1 << 54 | 1 << 63), 
    (0, 1 << 10 | 1 << 19 | 1 << 28 | 1 << 37 | 1 << 46 | 1 << 55), 
    (0, 1 << 11 | 1 << 20 | 1 << 29 | 1 << 38 | 1 << 47), 
    (0, 1 << 12 | 1 << 21 | 1 << 30 | 1 << 39), 
    (0, 1 << 13 | 1 << 22 | 1 << 31), 
    (0, 1 << 14 | 1 << 23), 
    (0, 1 << 15), 
    (0, 0), 
    (0, 1 << 17 | 1 << 26 | 1 << 35 | 1 << 44 | 1 << 53 | 1 << 62), 
    (1 << 0, 1 << 18 | 1 << 27 | 1 << 36 | 1 << 45 | 1 << 54 | 1 << 63), 
    (1 << 1, 1 << 19 | 1 << 28 | 1 << 37 | 1 << 46 | 1 << 55), 
    (1 << 2, 1 << 20 | 1 << 29 | 1 << 38 | 1 << 47), 
    (1 << 3, 1 << 21 | 1 << 30 | 1 << 39), 
    (1 << 4, 1 << 22 | 1 << 31), 
    (1 << 5, 1 << 23), 
    (1 << 6, 0), 
    (0, 1 << 25 | 1 << 34 | 1 << 43 | 1 << 52 | 1 << 61), 
    (1 << 8, 1 << 26 | 1 << 35 | 1 << 44 | 1 << 53 | 1 << 62), 
    (1 << 0 | 1 << 9, 1 << 27 | 1 << 36 | 1 << 45 | 1 << 54 | 1 << 63), 
    (1 << 1 | 1 << 10, 1 << 28 | 1 << 37 | 1 << 46 | 1 << 55), 
    (1 << 2 | 1 << 11, 1 << 29 | 1 << 38 | 1 << 47), 
    (1 << 3 | 1 << 12, 1 << 30 | 1 << 39), 
    (1 << 4 | 1 << 13, 1 << 31), 
    (1 << 5 | 1 << 14, 0), 
    (0, 1 << 33 | 1 << 42 | 1 << 51 | 1 << 60), 
    (1 << 16, 1 << 34 | 1 << 43 | 1 << 52 | 1 << 61), 
    (1 << 8 | 1 << 17, 1 << 35 | 1 << 44 | 1 << 53 | 1 << 62), 
    (1 << 0 | 1 << 9 | 1 << 18, 1 << 36 | 1 << 45 | 1 << 54 | 1 << 63), 
    (1 << 1 | 1 << 10 | 1 << 19, 1 << 37 | 1 << 46 | 1 << 55), 
    (1 << 2 | 1 << 11 | 1 << 20, 1 << 38 | 1 << 47), 
    (1 << 3 | 1 << 12 | 1 << 21, 1 << 39), 
    (1 << 4 | 1 << 13 | 1 << 22, 0), 
    (0, 1 << 41 | 1 << 50 | 1 << 59), 
    (1 << 24, 1 << 42 | 1 << 51 | 1 << 60), 
    (1 << 16 | 1 << 25, 1 << 43 | 1 << 52 | 1 << 61), 
    (1 << 8 | 1 << 17 | 1 << 26, 1 << 44 | 1 << 53 | 1 << 62), 
    (1 << 0 | 1 << 9 | 1 << 18 | 1 << 27, 1 << 45 | 1 << 54 | 1 << 63), 
    (1 << 1 | 1 << 10 | 1 << 19 | 1 << 28, 1 << 46 | 1 << 55), 
    (1 << 2 | 1 << 11 | 1 << 20 | 1 << 29, 1 << 47), 
    (1 << 3 | 1 << 12 | 1 << 21 | 1 << 30, 0), 
    (0, 1 << 49 | 1 << 58), 
    (1 << 32, 1 << 50 | 1 << 59), 
    (1 << 24 | 1 << 33, 1 << 51 | 1 << 60), 
    (1 << 16 | 1 << 25 | 1 << 34, 1 << 52 | 1 << 61), 
    (1 << 8 | 1 << 17 | 1 << 26 | 1 << 35, 1 << 53 | 1 << 62), 
    (1 << 0 | 1 << 9 | 1 << 18 | 1 << 27 | 1 << 36, 1 << 54 | 1 << 63), 
    (1 << 1 | 1 << 10 | 1 << 19 | 1 << 28 | 1 << 37, 1 << 55), 
    (1 << 2 | 1 << 11 | 1 << 20 | 1 << 29 | 1 << 38, 0), 
    (0, 1 << 57), 
    (1 << 40, 1 << 58), 
    (1 << 32 | 1 << 41, 1 << 59), 
    (1 << 24 | 1 << 33 | 1 << 42, 1 << 60), 
    (1 << 16 | 1 << 25 | 1 << 34 | 1 << 43, 1 << 61), 
    (1 << 8 | 1 << 17 | 1 << 26 | 1 << 35 | 1 << 44, 1 << 62), 
    (1 << 0 | 1 << 9 | 1 << 18 | 1 << 27 | 1 << 36 | 1 << 45, 1 << 63), 
    (1 << 1 | 1 << 10 | 1 << 19 | 1 << 28 | 1 << 37 | 1 << 46, 0), 
    (0, 0), 
    (1 << 48, 0), 
    (1 << 40 | 1 << 49, 0), 
    (1 << 32 | 1 << 41 | 1 << 50, 0), 
    (1 << 24 | 1 << 33 | 1 << 42 | 1 << 51, 0), 
    (1 << 16 | 1 << 25 | 1 << 34 | 1 << 43 | 1 << 52, 0), 
    (1 << 8 | 1 << 17 | 1 << 26 | 1 << 35 | 1 << 44 | 1 << 53, 0), 
    (1 << 0 | 1 << 9 | 1 << 18 | 1 << 27 | 1 << 36 | 1 << 45 | 1 << 54, 0), 
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
            diag |= 1 << x0 + i + (y0 - i) * 8;
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
            diag |= 1 << x0 + i + (y0 + i) * 8;
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
                let s = "1 << ".to_string() + &lsb.to_string();
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
                let s = "1 << ".to_string() + &lsb.to_string();
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
        let blockers: u64 = 1 << 20;
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
        let expected = (1 << 0)
            | (1 << 9)
            | (1 << 18)
            | (1 << 36)
            | (1 << 45)
            | (1 << 54)
            | (1 << 63)
            | (1 << 6)
            | (1 << 13)
            | (1 << 20)
            | (1 << 34)
            | (1 << 41)
            | (1 << 48);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_edge_of_board() {
        let index = 0;
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected =
            (1 << 9) | (1 << 18) | (1 << 27) | (1 << 36) | (1 << 45) | (1 << 54) | (1 << 63);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_corner_case() {
        let index = 63;
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected =
            (1 << 54) | (1 << 45) | (1 << 36) | (1 << 27) | (1 << 18) | (1 << 9) | (1 << 0);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_central_position() {
        let index = 28;
        let blockers = 0;
        let moves = bishop_moves(index, blockers);
        let expected = (1 << 19)
            | (1 << 10)
            | (1 << 1)
            | (1 << 37)
            | (1 << 46)
            | (1 << 55)
            | (1 << 56 | 1 << 49 | 1 << 42 | 1 << 35 | 1 << 21 | 1 << 14 | 1 << 7);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_single_blocker() {
        let index = 36;
        let blockers = 1 << 27;
        let moves = bishop_moves(index, blockers);
        let expected = (1 << 45)
            | (1 << 54)
            | (1 << 63)
            | (1 << 27)
            | (1 << 15)
            | (1 << 22)
            | (1 << 29)
            | (1 << 43)
            | (1 << 50 | 1 << 57);
        assert_eq!(moves, expected);
    }

    #[test]
    fn test_table_bishop_multiple_blockers() {
        let index = 36;
        let blockers = (1 << 27) | (1 << 45);
        let moves = bishop_moves(index, blockers);
        let expected =
            (1 << 27) | (1 << 45) | (1 << 15) | (1 << 22 | 1 << 29 | 1 << 43 | 1 << 50 | 1 << 57);
        assert_eq!(moves, expected);
    }
}
