use actix::prelude::*;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;

use crate::board::bitboard::piece_move::GenMoves;
use crate::board::bitboard::{self, BitBoardMove};

pub trait Engine {
    // start thinking
    fn go(&mut self, bit_position: bitboard::BitPosition);
    // stop thinking
    fn stop(&self);
}
pub trait EngineActor:
    Actor
    + Engine
    + Default
    + Clone
    + Handler<EngineGo>
    + Handler<EngineGetBestMove>
    + Actor<Context = actix::Context<Self>>
{
}

// Implementation
#[derive(Debug, Clone, Default)]
pub struct EngineDummy {
    best_move: Option<BitBoardMove>,
    rng: ThreadRng,
}
pub fn gen_moves(bit_position: bitboard::BitPosition) -> Vec<bitboard::BitBoardMove> {
    let bit_boards_white_and_black = bit_position.bit_boards_white_and_black();
    let bit_position_status = bit_position.bit_position_status();
    let color = &bit_position_status.player_turn();
    let check_status = bit_boards_white_and_black.check_status(&color);
    let capture_en_passant = bit_position_status.pawn_en_passant();
    let moves = bit_boards_white_and_black.gen_moves_for_all(
        color,
        check_status,
        &capture_en_passant,
        bit_position_status,
    );
    moves
}

impl Actor for EngineDummy {
    type Context = Context<Self>;
}
impl EngineActor for EngineDummy {}
impl Engine for EngineDummy {
    fn go(&mut self, bit_position: bitboard::BitPosition) {
        println!("EngineDummy started thinking.");
        let moves = gen_moves(bit_position);
        self.best_move = moves.choose(&mut self.rng).cloned();
    }
    fn stop(&self) {
        println!("EngineDummy stopped thinking.");
    }
}

#[derive(Message, Default)]
#[rtype(result = "Option<bitboard::BitBoardMove>")]
pub struct EngineGetBestMove {}
impl Handler<EngineGetBestMove> for EngineDummy {
    type Result = Option<bitboard::BitBoardMove>;

    fn handle(&mut self, _msg: EngineGetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        self.best_move
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineGo {
    bit_position: bitboard::BitPosition,
}
impl EngineGo {
    pub fn new(bit_position: bitboard::BitPosition) -> Self {
        EngineGo { bit_position }
    }
}
impl Handler<EngineGo> for EngineDummy {
    type Result = ();

    fn handle(&mut self, msg: EngineGo, _ctx: &mut Self::Context) {
        self.go(msg.bit_position);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineStop;

impl Handler<EngineStop> for EngineDummy {
    type Result = ();

    fn handle(&mut self, _msg: EngineStop, _ctx: &mut Self::Context) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::game::{self, tests::build_game_actor};

    #[actix::test]
    async fn test_engine_dummy() {
        let mut best_moves = Vec::new();
        let inputs = vec!["position startpos", "go"];

        for _ in 0..10 {
            let game_actor = build_game_actor(inputs.clone()).await;
            let best_move = game_actor
                .send(game::GetBestMove)
                .await
                .expect("actix mailbox error") // Ensure no Actix mailbox error
                .expect("No best move found"); // Ensure a best move is found

            let best_move_str = best_move.cast(); // Convert the best move to the desired format (if necessary)
            best_moves.push(best_move_str); // Add the best move to the Vec
        }
        let unique_moves: HashSet<_> = best_moves.iter().cloned().collect();
        // ensure that we generate random moves
        assert!(unique_moves.len() > 1)
    }
}
