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

// apply inverse_projection to LEFT_MOVES 
#[rustfmt::skip]
const DOWN_MOVES: [u64; 128] = [
    0x1010101010101, 0x1010101010101, 0x1010101010100, 0x1010101010100, 0x1010101010000, 0x1010101010000, 0x1010101010000, 0x1010101010000, 
    0x1010101000000, 0x1010101000000, 0x1010101000000, 0x1010101000000, 0x1010101000000, 0x1010101000000, 0x1010101000000, 0x1010101000000, 
    0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 
    0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 0x1010100000000, 
    0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 
    0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 
    0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 
    0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 0x1010000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
    0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 0x1000000000000, 
];

// apply inverse_projection to RIGHT_MOVES 
#[rustfmt::skip]
const UP_MOVES: [u64; 128] = [
    0x1010101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x10101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1000101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x10101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x101010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
    0x1010101, 0x1, 0x101, 0x1, 0x10101, 0x1, 0x101, 0x1, 
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
        7 => LEFT_MOVES[blockers as usize],
        _ => panic!("For rook, col {col} should not happen."),
    }
}

const MASK_ROW_0: u64 = 255;
const MASK_ROW_1: u64 = MASK_ROW_0 << 8;
const MASK_ROW_2: u64 = MASK_ROW_0 << 16;
const MASK_ROW_3: u64 = MASK_ROW_0 << 24;
const MASK_ROW_4: u64 = MASK_ROW_0 << 32;
const MASK_ROW_5: u64 = MASK_ROW_0 << 40;

pub fn table_rook_v(row: u8, blockers: u64) -> u64 {
    // map vertical to horizontal axis
    let blockers: u8 = (blockers & 1
        | (blockers >> 7) & 2
        | (blockers >> 14) & 4
        | (blockers >> 21) & 8
        | (blockers >> 28) & 16
        | (blockers >> 35) & 32
        | (blockers >> 42) & 64
        | (blockers >> 49) & 128) as u8;
    match row {
        0 => UP_MOVES[((blockers & 254) as usize) >> 8] << 8,
        1 => {
            DOWN_MOVES[(blockers & 1) as usize] & MASK_ROW_0
                | UP_MOVES[((blockers & 252) as usize) >> 16] << 16
        }
        2 => {
            DOWN_MOVES[(blockers & 3) as usize] & (MASK_ROW_0 | MASK_ROW_1)
                | UP_MOVES[((blockers & 248) as usize) >> 3] << 24
        }
        3 => {
            DOWN_MOVES[(blockers & 7) as usize] & (MASK_ROW_0 | MASK_ROW_1 | MASK_ROW_2)
                | UP_MOVES[((blockers & 240) as usize) >> 4] << 32
        }
        4 => {
            DOWN_MOVES[(blockers & 15) as usize] & (MASK_ROW_0 | MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3)
                | UP_MOVES[((blockers & 224) as usize) >> 5] << 40
        }
        5 => {
            DOWN_MOVES[(blockers & 31) as usize] & (MASK_ROW_0 | MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4)
                | UP_MOVES[((blockers & 192) as usize) >> 6] << 48
        }
        6 => {
            DOWN_MOVES[(blockers & 63) as usize] & (MASK_ROW_0 | MASK_ROW_1 | MASK_ROW_2 | MASK_ROW_3 | MASK_ROW_4 | MASK_ROW_5)
                | UP_MOVES[((blockers & 128) as usize) >> 7] << 56
        }
        7 => DOWN_MOVES[blockers as usize],
        _ => panic!("For rook, row {row} should not happen."),
    }
}

mod tests {
    use super::*;

    fn inverse_projection(projection: u64) -> u64 {
        (projection & 1)
            | ((projection & 2) << 7)
            | ((projection & 4) << 14)
            | ((projection & 8) << 21)
            | ((projection & 16) << 28)
            | ((projection & 32) << 35)
            | ((projection & 64) << 42)
            | ((projection & 128) << 49)
    }

    #[test]
    #[ignore]
    fn show_up_moves_inv() {
        let up_moves_inv = RIGHT_MOVES.map(|projection| inverse_projection(projection as u64));
        for i in 0..up_moves_inv.len() {
            print!("0x{:x}, ", up_moves_inv[i]);
            if (i + 1) % 8 == 0 {
                println!();
            }
        }
    }

    #[test]
    #[ignore]
    fn show_down_moves_inv() {
        let down_moves_inv = DOWN_MOVES.map(|projection| inverse_projection(projection as u64));
        for i in 0..down_moves_inv.len() {
            print!("0x{:x}, ", down_moves_inv[i]);
            if (i + 1) % 8 == 0 {
                println!();
            }
        }
    }
}
