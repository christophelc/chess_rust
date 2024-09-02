pub mod table_bishop;
pub mod table_rook;

const MASK_ROW_0: u64 = 255;
const MASK_ROW_1: u64 = MASK_ROW_0 << 8;
const MASK_ROW_2: u64 = MASK_ROW_0 << 16;
const MASK_ROW_3: u64 = MASK_ROW_0 << 24;
const MASK_ROW_4: u64 = MASK_ROW_0 << 32;
const MASK_ROW_5: u64 = MASK_ROW_0 << 40;
const MASK_ROW_6: u64 = MASK_ROW_0 << 48;
const MASK_ROW_7: u64 = MASK_ROW_0 << 56;

fn check_bit(index: u8, value: u64) -> u8 {
    if (1 << index) & value == 0 {
        0
    } else {
        1
    }
}
