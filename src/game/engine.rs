use rand_chacha::ChaCha12Rng;
use tokio::task::spawn_local;
use tokio::time::{sleep, Duration};

use actix::prelude::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::board::bitboard::piece_move::GenMoves;
use crate::board::bitboard::{self, BitBoardMove};

#[derive(Debug)]
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
    fn start_event_loop(&mut self);
    // stop thinkin
    fn stop_event_loop(&mut self);

    fn set_is_running(&mut self, is_running: bool);

    fn set_is_thinking(&mut self, is_thinking: bool);

    fn is_running(&self) -> bool;

    fn is_thinking(&self) -> bool;

    // main loop for thinking
    async fn event_loop(&self) {
        while self.is_running() {
            sleep(Duration::from_millis(100)).await;
            //println!("is_thinking: {}", self.is_thinking());
            if self.is_thinking() {
                self.think().await;
            }
        }
        println!("Event loop has stopped for Engine id {:?}.", self.id());
    }

    async fn think(&self);

    // start thinking
    fn start_thinking(&mut self, bit_position: bitboard::BitPosition);

    // stop thinking
    fn stop_thinking(&mut self);
}
pub trait EngineActor:
    Actor
    + Engine
    + Default
    + Clone
    + Send
    + Handler<EngineStartThinking>
    + Handler<EngineGetBestMove>
    + Handler<EngineGetId>
    + Handler<EngineStopThinking>
    + Handler<EngineCleanResources>
    + Actor<Context = actix::Context<Self>>
{
}

// Implementation
#[derive(Debug, Clone, Default)]
pub struct EngineDummy {
    addr: Option<Addr<EngineDummy>>,
    bit_position_opt: Option<bitboard::BitPosition>, // initial position to be played
    best_move: Option<BitBoardMove>,
    engine_status: EngineStatus,
    id_number: String,
}
impl EngineDummy {
    pub fn set_id_number(&self, id_number: &str) -> Self {
        Self {
            id_number: id_number.to_string(),
            ..self.clone()
        }
    }
}
unsafe impl Send for EngineDummy {}

const DUMMY_ENGINE_ID_NAME: &str = "Random engine";
const DUMMY_ENGINE_ID_AUTHOR: &str = "Christophe le cam";

pub fn gen_moves(bit_position: &bitboard::BitPosition) -> Vec<bitboard::BitBoardMove> {
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

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.addr = Some(ctx.address());
        self.start_event_loop();
    }
    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        self.stop_event_loop();
    }
}
impl EngineActor for EngineDummy {}
impl Engine for EngineDummy {
    fn id(&self) -> EngineId {
        EngineId {
            name: format!("{} {}", DUMMY_ENGINE_ID_NAME.to_owned(), self.id_number)
                .trim()
                .to_string(),
            author: DUMMY_ENGINE_ID_AUTHOR.to_owned(),
        }
    }
    fn set_is_running(&mut self, is_running: bool) {
        self.engine_status = self.engine_status.clone().set_is_running(is_running);
    }
    fn set_is_thinking(&mut self, is_thinking: bool) {
        self.engine_status = self.engine_status.clone().set_is_thinking(is_thinking);
    }
    fn start_event_loop(&mut self) {
        // we can start an event loop only one time
        if !self.is_running() {
            self.set_is_running(true);
            let self_ref = self as *mut Self;
            spawn_local(async move {
                let self_ref = unsafe { &mut *self_ref }; // Dereference raw pointer
                self_ref.event_loop().await;
            });
            println!("Event loop started for Engine id {:?}.", self.id());
        }
    }
    fn stop_event_loop(&mut self) {
        if self.is_running() {
            self.set_is_running(false);
            println!("Event loop stopped for Engine id {:?}.", self.id());
        }
    }
    fn is_running(&self) -> bool {
        self.engine_status.is_running()
    }
    fn is_thinking(&self) -> bool {
        self.engine_status.is_thinking()
    }
    async fn think(&self) {
        // first generate moves
        let moves = gen_moves(self.bit_position_opt.as_ref().expect("Missing position."));
        // And then stop thinking and clear positino
        self.addr
            .as_ref()
            .unwrap()
            .send(EngineStopThinking)
            .await
            .expect("Actix error");
        let mut rng = ChaCha12Rng::from_entropy();
        let best_move = moves.choose(&mut rng).cloned();
        let result = self
            .addr
            .as_ref()
            .unwrap()
            .send(EngineSetBestMove(best_move))
            .await;
        result.unwrap();
    }
    fn start_thinking(&mut self, bit_position: bitboard::BitPosition) {
        assert!(self.is_running() && !self.is_thinking());
        self.bit_position_opt = Some(bit_position);
        println!("EngineDummy of id {:?} started thinking.", self.id());
        self.engine_status = self.engine_status.clone().set_is_thinking(true);
    }
    fn stop_thinking(&mut self) {
        if self.is_thinking() {
            self.set_is_thinking(false);
            self.bit_position_opt = None;
            println!("EngineDummy of id {:?} stopped thinking.", self.id());
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct EngineStatus {
    is_running: bool,  // event loop should always be running
    is_thinking: bool, // thinking
}
impl EngineStatus {
    #[cfg(test)]
    pub fn new(is_thinking: bool, is_running: bool) -> Self {
        Self {
            is_thinking,
            is_running,
        }
    }
    pub fn is_thinking(&self) -> bool {
        self.is_thinking
    }
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    pub fn set_is_thinking(&self, is_thinking: bool) -> Self {
        Self {
            is_thinking,
            is_running: self.is_running,
        }
    }
    pub fn set_is_running(&self, is_running: bool) -> Self {
        Self {
            is_running,
            is_thinking: self.is_thinking,
        }
    }
}
#[derive(Message, Default)]
#[rtype(result = "Option<EngineId>")]
pub struct EngineGetId;
impl Handler<EngineGetId> for EngineDummy {
    type Result = Option<EngineId>;

    fn handle(&mut self, _msg: EngineGetId, _ctx: &mut Self::Context) -> Self::Result {
        Some(self.id())
    }
}

#[derive(Message, Default)]
#[rtype(result = "Option<EngineStatus>")]
pub struct EngineGetStatus;
impl Handler<EngineGetStatus> for EngineDummy {
    type Result = Option<EngineStatus>;

    fn handle(&mut self, _msg: EngineGetStatus, _ctx: &mut Self::Context) -> Self::Result {
        Some(self.engine_status.clone())
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
pub struct EngineSetBestMove(Option<bitboard::BitBoardMove>);
impl Handler<EngineSetBestMove> for EngineDummy {
    type Result = ();

    fn handle(&mut self, msg: EngineSetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        self.best_move = msg.0;
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineSetStatus(EngineStatus);
impl Handler<EngineSetStatus> for EngineDummy {
    type Result = ();

    fn handle(&mut self, msg: EngineSetStatus, _ctx: &mut Self::Context) -> Self::Result {
        self.engine_status = msg.0;
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineStartThinking {
    bit_position: bitboard::BitPosition,
}
impl EngineStartThinking {
    pub fn new(bit_position: bitboard::BitPosition) -> Self {
        EngineStartThinking { bit_position }
    }
}
impl Handler<EngineStartThinking> for EngineDummy {
    type Result = ();

    fn handle(&mut self, msg: EngineStartThinking, _ctx: &mut Self::Context) {
        self.start_thinking(msg.bit_position);
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineStopThinking;

impl Handler<EngineStopThinking> for EngineDummy {
    type Result = ();

    fn handle(&mut self, _msg: EngineStopThinking, _ctx: &mut Self::Context) {
        self.stop_thinking();
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineCleanResources;

impl Handler<EngineCleanResources> for EngineDummy {
    type Result = ();

    fn handle(&mut self, _msg: EngineCleanResources, _ctx: &mut Self::Context) {
        self.stop_event_loop();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::game::{engine::EngineGetId, game_manager, game_manager::build_game_actor};

    #[actix::test]
    async fn test_engine_dummy() {
        let inputs = vec!["position startpos", "go", "quit"];
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
        let inputs = vec!["position startpos", "go", "wait100ms", "quit"];

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
