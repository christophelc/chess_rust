#![allow(warnings)]

use anyhow::{anyhow, Result};
use scryer_prolog::{LeafAnswer, Machine, QueryState, Term};
//use scryer_prolog::MachineBuilder;
use std::{collections::BTreeMap, fs};

use crate::entity::game::component::{
    bitboard::{
        zobrist::{self, ZobristHash},
        BitPosition,
    },
    game_state::GameState,
    square::{Color, Piece, TypePiece},
};

// ------------ Prolog program (unchanged) ------------
fn prolog_program() -> std::io::Result<String> {
    let content = fs::read_to_string("prolog/training/krk.pl")?;
    Ok(content)
}

// ------------ Helpers ------------
fn int_from_term(t: &Term) -> Result<i32> {
    match t {
        Term::Integer(bi) => Ok(bi.to_string().parse::<i32>()?),
        _ => Err(anyhow!("expected integer, got: {t:?}")),
    }
}

fn get_i(bindings: &BTreeMap<String, Term>, name: &str) -> Result<i32> {
    let t = bindings
        .get(name)
        .ok_or_else(|| anyhow!("missing variable {name}"))?;
    int_from_term(t)
}

// ------------ Generic borrowed iterator over a Prolog query ------------
pub struct PlIter<'a> {
    answers: QueryState<'a>, // borrows the machine
}

impl<'a> PlIter<'a> {
    pub fn new(machine: &'a mut Machine, query: &str) -> Self {
        let answers = machine.run_query(query);
        Self { answers }
    }
}

impl<'a> Iterator for PlIter<'a> {
    // Yield one set of bindings per solution
    type Item = Result<BTreeMap<String, Term>>;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.answers.next()?;
        let leaf = match next {
            Ok(l) => l,
            Err(e) => return Some(Err(anyhow!("Prolog error: {e:?}"))),
        };

        match leaf {
            LeafAnswer::LeafAnswer { bindings, .. } => Some(Ok(bindings)), // move out
            LeafAnswer::True => Some(Err(anyhow!("no variables (True)"))),
            LeafAnswer::False => None, // end of search
            LeafAnswer::Exception(t) => Some(Err(anyhow!("exception: {t:?}"))),
        }
    }
}

// ------------ KRK-specific iterator built on top of PlIter ------------
pub struct KrkIter<'a> {
    inner: PlIter<'a>,
    zobrist_table: zobrist::Zobrist,
    is_white_turn: bool,
}

impl<'a> KrkIter<'a> {
    pub fn new(machine: &'a mut Machine, is_white_turn: bool) -> Self {
        Self {
            inner: PlIter::new(machine, "krk(WK, WR, BK)."),
            zobrist_table: zobrist::Zobrist::new(),
            is_white_turn,
        }
    }

    /// If you ever want to tweak the query (e.g., add constraints),
    /// you can pass a different one here without changing the iterator type.
    pub fn with_query(machine: &'a mut Machine, white_turn: bool, query: &str) -> Self {
        Self {
            inner: PlIter::new(machine, query),
            zobrist_table: zobrist::Zobrist::new(),
            is_white_turn: white_turn,
        }
    }
}

pub fn check_valid_bitboard(bit_position: &BitPosition, zobrist_table: &zobrist::Zobrist) -> bool {
    // The other side should not be in check
    let mut bit_position_other_side = bit_position.clone();
    bit_position_other_side.change_side();
    let game_state = GameState::new(bit_position_other_side.to(), zobrist_table);
    let check_status = game_state.check_status();
    !check_status.is_check()
}

impl<'a> Iterator for KrkIter<'a> {
    type Item = Result<BitPosition>; // or your concrete error type

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Drive the inner iterator
            let bindings = match self.inner.next() {
                Some(Ok(b)) => b,
                Some(Err(e)) => return Some(Err(e)), // surface inner error
                None => return None,                 // end of iteration
            };

            // Build the position
            let mut bit_position = BitPosition::empty();
            let mut zobrist_hash = ZobristHash::default();

            let white_king = Piece::new(TypePiece::King, Color::White);
            let black_king = Piece::new(TypePiece::King, Color::Black);
            let white_rook = Piece::new(TypePiece::Rook, Color::White);

            let wk = match get_i(&bindings, "WK") {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let wr = match get_i(&bindings, "WR") {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            let bk = match get_i(&bindings, "BK") {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };

            bit_position.add_piece_without_control(
                white_king,
                wk as u8,
                &mut zobrist_hash,
                &self.zobrist_table,
            );
            bit_position.add_piece_without_control(
                white_rook,
                wr as u8,
                &mut zobrist_hash,
                &self.zobrist_table,
            );
            bit_position.add_piece_without_control(
                black_king,
                bk as u8,
                &mut zobrist_hash,
                &self.zobrist_table,
            );

            // we don't verify check if black turn since prolog already ensures kings distant enough
            if !self.is_white_turn || check_valid_bitboard(&bit_position, &self.zobrist_table) {
                return Some(Ok(bit_position));
            } else {
                // invalid according to your check → SKIP and continue pulling
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use scryer_prolog::MachineBuilder;

    fn read_prolog_program() -> String {
        prolog_program().expect("Erreur when reading Prolog program")
    }

    #[test]
    fn check_control() {
        let mut bit_position = BitPosition::empty();
        let mut zobrist_hash = ZobristHash::default();
        let zobrist_table = zobrist::Zobrist::new();

        let white_king = Piece::new(TypePiece::King, Color::White);
        let black_king = Piece::new(TypePiece::King, Color::Black);
        let white_rook = Piece::new(TypePiece::Rook, Color::White);
        let wk = 0u64;
        let bk = 63u64;
        let wr = 7u64;
        bit_position.add_piece_without_control(
            white_king,
            wk as u8,
            &mut zobrist_hash,
            &zobrist_table,
        );
        bit_position.add_piece_without_control(
            white_rook,
            wr as u8,
            &mut zobrist_hash,
            &zobrist_table,
        );
        bit_position.add_piece_without_control(
            black_king,
            bk as u8,
            &mut zobrist_hash,
            &zobrist_table,
        );
        assert!(!check_valid_bitboard(&bit_position, &zobrist_table))
    }

    // no really useful test since we have cast result into a BitPosition anyway
    #[ignore]
    #[test]
    fn take_first_five_krk_positions() -> Result<()> {
        let mut machine = MachineBuilder::new().build();
        machine.consult_module_string("user", &read_prolog_program());

        let it = KrkIter::new(&mut machine, true);

        // Collect first 5 solutions as a Vec, propagating any Prolog/parse error
        let v: Vec<BitPosition> = it.take(5).collect::<Result<Vec<_>>>()?;

        assert_eq!(v.len(), 5, "Prolog must return five positions");

        // Check constraints over returned bitboards
        for bitboard in &v {
            let wk = bitboard
                .bit_boards_white_and_black()
                .bit_board_white()
                .king()
                .bitboard()
                .index()
                .value() as i32;
            let wr = bitboard
                .bit_boards_white_and_black()
                .bit_board_white()
                .rooks()
                .bitboard()
                .index()
                .value() as i32;
            let bk = bitboard
                .bit_boards_white_and_black()
                .bit_board_black()
                .king()
                .bitboard()
                .index()
                .value() as i32;

            // squares are 0..=63
            assert!((0..=63).contains(&wk));
            assert!((0..=63).contains(&wr));
            assert!((0..=63).contains(&bk));

            // distinct squares
            assert!(wk != wr && wk != bk && wr != bk);

            // kings not adjacent (Chebyshev distance != 1)
            let (wf, wrk) = (wk % 8, wk / 8);
            let (bf, brk) = (bk % 8, bk / 8);
            let df = (wf - bf).abs();
            let dr = (wrk - brk).abs();
            assert!(df.max(dr) != 1);
        }

        Ok(())
    }

    #[ignore]
    #[test]
    fn count_krk_positions_black_turn() -> () {
        let mut machine = MachineBuilder::new().build();
        machine.consult_module_string("user", &read_prolog_program());
        let it = KrkIter::new(&mut machine, false);
        let n = it.count();
        // no filter positions where black is in check
        assert_eq!(n, 223944)
    }

    #[ignore]
    #[test]
    fn count_krk_positions_white_turn() -> () {
        let mut machine = MachineBuilder::new().build();
        machine.consult_module_string("user", &read_prolog_program());
        let it = KrkIter::new(&mut machine, true);
        let n = it.count();
        // filter positions where black is in check
        assert_eq!(n, 175168)
    }
}
