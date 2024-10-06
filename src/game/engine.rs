use actix::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::board::bitboard::piece_move::GenMoves;
use crate::board::bitboard::{self, BitBoardMove};

pub struct EngineId {
    name: String,
    author: String,
}
impl EngineId {
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn author(&self) -> String {
        self.author.clone()
    }
}
pub trait Engine {
    fn id(&self) -> EngineId;

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
    + Send
    + Handler<EngineGo>
    + Handler<EngineGetBestMove>
    + Handler<EngineGetId>
    + Actor<Context = actix::Context<Self>>
{
}

// Implementation
#[derive(Debug, Clone, Default)]
pub struct EngineDummy {
    best_move: Option<BitBoardMove>,
}
const DUMMY_ENGINE_ID_NAME: &str = "Random engine";
const DUMMY_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

pub fn gen_moves(bit_position: bitboard::BitPosition) -> Vec<bitboard::BitBoardMove> {
    let bit_boards_white_and_black = bit_position.bit_boards_white_and_black();
    let bit_position_status = bit_position.bit_position_status();
    let color = &bit_position_status.player_turn();
    let check_status = bit_boards_white_and_black.check_status(color);
    let capture_en_passant = bit_position_status.pawn_en_passant();
    bit_boards_white_and_black.gen_moves_for_all(
        color,
        check_status,
        &capture_en_passant,
        bit_position_status,
    )
}

impl Actor for EngineDummy {
    type Context = Context<Self>;
}
impl EngineActor for EngineDummy {}
impl Engine for EngineDummy {
    fn id(&self) -> EngineId {
        EngineId {
            name: DUMMY_ENGINE_ID_NAME.to_owned(),
            author: DUMMY_ENGINE_ID_AUTHOR.to_owned(),
        }
    }
    fn go(&mut self, bit_position: bitboard::BitPosition) {
        println!("EngineDummy started thinking.");
        let moves = gen_moves(bit_position);
        let mut rng = thread_rng();
        self.best_move = moves.choose(&mut rng).cloned();
    }
    fn stop(&self) {
        println!("EngineDummy stopped thinking.");
    }
}

#[derive(Message, Default)]
#[rtype(result = "Option<EngineId>")]
pub struct EngineGetId {}
impl Handler<EngineGetId> for EngineDummy {
    type Result = Option<EngineId>;

    fn handle(&mut self, _msg: EngineGetId, _ctx: &mut Self::Context) -> Self::Result {
        Some(self.id())
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

    use crate::game::{engine::EngineGetId, game_manager, game_manager::build_game_actor};

    #[actix::test]
    async fn test_engine_dummy() {
        let inputs = vec!["position startpos", "go"];
        let game_actor = build_game_actor(inputs.clone()).await;
        let msg = game_manager::GetCurrentEngine::default();
        let result = game_actor.send(msg).await;
        let mut vec_engine_id: Vec<String> = vec![];
        if let Ok(Some(engine_actor)) = result {
            let engine_id_opt = engine_actor.send(EngineGetId::default()).await;
            if let Ok(Some(engine_id)) = engine_id_opt {
                vec_engine_id.push(engine_id.name().to_string());
                vec_engine_id.push(engine_id.author().to_string());
            }
        }
        assert_eq!(vec_engine_id, vec!["Random engine", "Christophe le cam"])
    }

    #[actix::test]
    async fn test_engine_dummy_is_random() {
        let mut best_moves = Vec::new();
        let inputs = vec!["position startpos", "go"];

        for _ in 0..10 {
            let game_actor = build_game_actor(inputs.clone()).await;
            let best_move = game_actor
                .send(game_manager::GetBestMove)
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
