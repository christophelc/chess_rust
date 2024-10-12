const MASK_ROOK: [u64; 64] = [
    0x302,
    0x705,
    0xe0a,
    0x1c14,
    0x3828,
    0x7050,
    0xe0a0,
    0xc040,
    0x30203,
    0x70507,
    0xe0a0e,
    0x1c141c,
    0x382838,
    0x705070,
    0xe0a0e0,
    0xc040c0,
    0x3020300,
    0x7050700,
    0xe0a0e00,
    0x1c141c00,
    0x38283800,
    0x70507000,
    0xe0a0e000,
    0xc040c000,
    0x302030000,
    0x705070000,
    0xe0a0e0000,
    0x1c141c0000,
    0x3828380000,
    0x7050700000,
    0xe0a0e00000,
    0xc040c00000,
    0x30203000000,
    0x70507000000,
    0xe0a0e000000,
    0x1c141c000000,
    0x382838000000,
    0x705070000000,
    0xe0a0e0000000,
    0xc040c0000000,
    0x3020300000000,
    0x7050700000000,
    0xe0a0e00000000,
    0x1c141c00000000,
    0x38283800000000,
    0x70507000000000,
    0xe0a0e000000000,
    0xc040c000000000,
    0x302030000000000,
    0x705070000000000,
    0xe0a0e0000000000,
    0x1c141c0000000000,
    0x3828380000000000,
    0x7050700000000000,
    0xe0a0e00000000000,
    0xc040c00000000000,
    0x203000000000000,
    0x507000000000000,
    0xa0e000000000000,
    0x141c000000000000,
    0x2838000000000000,
    0x5070000000000000,
    0xa0e0000000000000,
    0x40c0000000000000,
];

pub fn king_moves(index: u8) -> u64 {
    MASK_ROOK[index as usize]
}

mod tests {

    #[allow(dead_code)]
    fn gen_move_king_at(index: u8) -> u64 {
        let is_row_1 = index < 8;
        let is_col_a = index % 8 == 0;
        let is_row_8 = index >= 56;
        let is_col_h = index % 8 == 7;
        let deltas: Vec<i8> = match (is_row_1, is_col_a, is_row_8, is_col_h) {
            // No edges or corners
            (false, false, false, false) => vec![-9, -8, -7, -1, 1, 7, 8, 9],
            // Single edges
            (false, false, false, true) => vec![-9, -8, -1, 7, 8],
            (false, false, true, false) => vec![-9, -8, -7, -1, 1],
            (false, true, false, false) => vec![-8, -7, 1, 8, 9],
            (true, false, false, false) => vec![-1, 1, 7, 8, 9],
            // Corners
            (true, true, false, false) => vec![1, 8, 9],
            (true, false, false, true) => vec![-1, 7, 8],
            (false, true, true, false) => vec![-8, -7, 1],
            (false, false, true, true) => vec![-9, -8, -1],
            // incompatible conditions: code never reached
            _ => vec![],
        };
        let mut moves_bitboard: u64 = 0;
        for &delta in deltas.iter() {
            let new_pos = index as i8 + delta;
            if (0..64).contains(&new_pos) {
                // Ensure within board bounds
                let pos = new_pos as u8;
                moves_bitboard |= 1u64 << pos;
            } else {
                panic!("This code should never be reached.")
            }
        }
        moves_bitboard
    }

    #[test]
    #[ignore]
    fn show_king_moves() {
        let mut moves: Vec<String> = Vec::new();
        for index in 0..64 {
            moves.push(format!("0x{:x}", gen_move_king_at(index)));
        }
        println!("[{}]", moves.join(","));
    }
}
