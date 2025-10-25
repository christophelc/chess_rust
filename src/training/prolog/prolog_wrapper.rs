use anyhow::{anyhow, Result};
use scryer_prolog::{LeafAnswer, Machine, MachineBuilder, QueryState, Term};
use std::collections::BTreeMap;

use crate::entity::game::component::{bitboard::{zobrist::{self, ZobristHash}, BitBoard, BitPosition}, game_state::GameState, square::{Color, Piece, TypePiece}};

// ------------ Helpers ------------
fn int_from_term(t: &Term) -> Result<i32> {
    match t {
        Term::Integer(bi) => Ok(bi.to_string().parse::<i32>()?),
        _ => Err(anyhow!("expected integer, got: {t:?}")),
    }
}

pub fn get_i(bindings: &BTreeMap<String, Term>, name: &str) -> Result<i32> {
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
            LeafAnswer::True  => Some(Err(anyhow!("no variables (True)"))),
            LeafAnswer::False => None, // end of search
            LeafAnswer::Exception(t) => Some(Err(anyhow!("exception: {t:?}"))),
        }
    }
}