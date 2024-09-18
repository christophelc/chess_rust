use crate::board::square;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LongAlgebricNotationMove {
    start: u8,
    end: u8,
    opt_promotion: Option<square::TypePiecePromotion>,
}
impl LongAlgebricNotationMove {
    pub fn new(start: u8, end: u8, opt_promotion: Option<square::TypePiecePromotion>) -> Self {
        LongAlgebricNotationMove {
            start,
            end,
            opt_promotion,
        }
    }
    pub fn build_from_str(move_str: &str) -> Result<Self, String> {
        let mut result = Err(format!("Invalid move: {}", move_str));
        if move_str.len() >= 4 && move_str.len() <= 5 {
            let from_square = &move_str[0..2]; // First two characters (e.g., "e2")
            let to_square = &move_str[2..4]; // Last two characters (e.g., "e4")
            let from_index = square_to_index(from_square);
            let to_index = square_to_index(to_square);
            let opt_promotion = promotion2type_piece(move_str.chars().nth(4))?;
            if from_index < 64 && to_index < 64 {
                result = Ok(LongAlgebricNotationMove::new(
                    from_index,
                    to_index,
                    opt_promotion,
                ));
            }
        }
        result
    }
    pub fn cast(&self) -> String {
        format!(
            "{}{}",
            index_to_string(self.start),
            index_to_string(self.end)
        )
    }
    pub fn start(&self) -> u8 {
        self.start
    }
    pub fn end(&self) -> u8 {
        self.end
    }
    pub fn opt_promotion(&self) -> Option<square::TypePiecePromotion> {
        self.opt_promotion
    }
}

fn index_to_string(index: u8) -> String {
    assert!(index < 64, "index '{}' should be < 64", index);
    let row = index / 8;
    let col = index % 8;
    format!("{}{}", col, row)
}

fn square_to_index(square: &str) -> u8 {
    let mut iter = square.chars();
    let col = iter.next().unwrap() as u8 - b'a'; // file 'a'-'h' -> 0-7
    let row = iter.next().unwrap().to_digit(10).unwrap() as u8 - 1; // rank '1'-'8' -> 0-7
    (row * 8) + col
}

fn promotion2type_piece(
    opt_promotion_as_char: Option<char>,
) -> Result<Option<square::TypePiecePromotion>, String> {
    match opt_promotion_as_char {
        None => Ok(None),
        Some('q') => Ok(Some(square::TypePiecePromotion::Queen)),
        Some('r') => Ok(Some(square::TypePiecePromotion::Rook)),
        Some('n') => Ok(Some(square::TypePiecePromotion::Knight)),
        Some('b') => Ok(Some(square::TypePiecePromotion::Bishop)),
        Some(p) => Err(format!(
            "Unknow promotion piece: '{}'. Valid pieces are: q, r, n",
            p
        )),
    }
}