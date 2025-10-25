use scryer_prolog::Machine;
use anyhow::Result;
use std::fs;
use crate::{entity::game::component::bitboard::BitIndex, training::prolog::prolog_wrapper::{get_i, PlIter}};

pub fn prolog_program() -> std::io::Result<String> {
    let content = fs::read_to_string("prolog/strategy/krk_strategy.pl")?;
    Ok(content)
}

pub struct KrkStrategy<'a> {
    inner: PlIter<'a>,
    is_white_turn: bool,
}

impl<'a> KrkStrategy<'a> {
    pub fn new(
        machine: &'a mut Machine, 
        wk: BitIndex,
        wr: BitIndex,
        bk: BitIndex,
        is_white_turn: bool) -> Self {
        Self {
            inner: PlIter::new(
                machine, 
                &format!("krk_strategy({}, {}, {}, MoveFrom, MoveTo).", wk.value(), wr.value(), bk.value())
            ), 
            is_white_turn,
        }
    }
    
    pub fn best_move(&mut self, program: &str) -> Option<Result<(i32, i32)>> {
        let bindings = match self.inner.next() {
            Some(Ok(b))  => b,
            Some(Err(e)) => return Some(Err(e)), // surface inner error
            None         => return None,         // end of iteration
        };
        let best_move_from = match get_i(&bindings, "MoveFrom") { Ok(v) => v, Err(e) => return Some(Err(e)) };
        let best_move_to = match get_i(&bindings, "MoveTo") { Ok(v) => v, Err(e) => return Some(Err(e)) };
        Some(Ok((best_move_from, best_move_to)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryer_prolog::MachineBuilder;

    fn read_prolog_program() -> String {
        prolog_program().expect("Erreur when reading Prolog program")
    }

    #[test]
    fn test_krk_strategy() -> () {
        let program = read_prolog_program();
        let mut machine = MachineBuilder::new().build();
        machine.consult_module_string("user", &read_prolog_program());
        let wk = BitIndex::new(60); // e8
        let wr = BitIndex::new(63); // h8
        let bk = BitIndex::new(4);  // e1
        let mut krk_strategy = KrkStrategy::new(&mut machine, wk, wr, bk, true);
        println!("{:?}", krk_strategy.best_move(&program))
        if let Some(Ok((from, to))) = krk_strategy.best_move(&program) {
            println!("Best move from {} to {}", from, to);
        } else {
            println!("No move found");  
        }
    }
}