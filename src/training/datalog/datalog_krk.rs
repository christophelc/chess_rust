use crepe::crepe;

crepe! {
    // --- Inputs ---
    // We seed the 64 squares of the board (0..=63) from Rust.
    @input
    struct Square(u8);

    struct SqFile(u8, u8); // (square, file)
    struct SqRank(u8, u8); // (square, rank)

    SqFile(s, s % 8) <- Square(s);
    SqRank(s, s / 8) <- Square(s);

    struct KingsAdjacent(u8, u8); // (s1, s2)
    KingsAdjacent(s1, s2) <-
        SqFile(s1, f1), SqRank(s1, r1),
        SqFile(s2, f2), SqRank(s2, r2),
        (
            ((f1 as i16 - f2 as i16).abs().max((r1 as i16 - r2 as i16).abs())) == 1
        );

    @output
    struct Krk(u8, u8, u8); // (WK, WR, BK)

    Krk(wk, wr, bk) <-
        Square(wk),
        Square(wr), (wr != wk),
        Square(bk), (bk != wk), (bk != wr),
        !KingsAdjacent(wk, bk); // stratified negation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_krk_count() {
        let mut rt = Crepe::new();
        rt.extend((0u8..=63).map(Square));
        let (krk,) = rt.run();

        // Display first 10 positions
        for Krk(wk, wr, bk) in krk.iter().take(10) {
            println!("WK={wk}, WR={wr}, BK={bk}");
        }
        // black turn => no filter positions where black is in check
        assert_eq!(krk.len(), 223944); 
    }
}
