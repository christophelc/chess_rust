use std::sync::Arc;

use rand_chacha::ChaCha12Rng;
use tokio::task::spawn_local;
use tokio::time::{sleep, Duration};

use actix::prelude::*;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::board::bitboard::piece_move::GenMoves;
use crate::board::bitboard::{self, BitBoardMove};
use crate::uci;

use super::game_manager;
use super::monitoring::debug;

#[derive(Debug, Clone)]
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

    fn think(&self, self_actor: Addr<EngineDispatcher>, bit_position: &bitboard::BitPosition);
}

////////////////////////////////////////////////////////
pub struct EngineDispatcher {
    engine: Arc<dyn Engine + Send + Sync>, // EngineActor dans un Arc
    debug_actor_opt: Option<debug::DebugActor>,
    engine_status: EngineStatus,
    ts_best_move_opt: Option<game_manager::TimestampedBitBoardMove>,
    self_actor_opt: Option<Addr<EngineDispatcher>>,
    game_manager_actor_opt: Option<game_manager::GameManagerActor>,
    bit_position_opt: Option<bitboard::BitPosition>, // initial position to be played
}
impl EngineDispatcher {
    pub fn new(
        engine: Arc<dyn Engine + Send + Sync>,
        debug_actor_opt: Option<debug::DebugActor>,
    ) -> Self {
        Self {
            engine,
            debug_actor_opt,
            engine_status: EngineStatus::default(),
            game_manager_actor_opt: None,
            ts_best_move_opt: None,
            self_actor_opt: None,
            bit_position_opt: None,
        }
    }
    fn get_addr(&self) -> Addr<EngineDispatcher> {
        self.self_actor_opt.as_ref().unwrap().clone()
    }
    fn get_best_move(&self) -> Option<game_manager::TimestampedBitBoardMove> {
        self.ts_best_move_opt.clone()
    }
    fn set_best_move(&mut self, best_move_opt: Option<BitBoardMove>) {
        self.ts_best_move_opt = best_move_opt.map(|best_move| {
            game_manager::TimestampedBitBoardMove::new(best_move, self.engine.id())
        });
    }
    // main loop for thinking
    async fn event_loop(&self) {
        while self.is_running() {
            sleep(Duration::from_millis(100)).await;
            if self.is_thinking() {
                self.engine
                    .think(self.get_addr(), self.bit_position_opt.as_ref().unwrap());
            }
        }
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "Event loop has stopped for Engine id {:?}.",
                self.engine.id()
            )));
        }
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
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "Event loop started for Engine id {:?}.",
                    self.engine.id()
                )))
            }
        }
    }
    fn stop_event_loop(&mut self) {
        if self.is_running() {
            self.set_is_running(false);
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "Event loop stopped for Engine id {:?}.",
                    self.engine.id()
                )));
            }
        }
    }
    fn set_is_running(&mut self, is_running: bool) {
        self.engine_status = self.engine_status.clone().set_is_running(is_running);
    }
    fn set_is_thinking(&mut self, is_thinking: bool) {
        if self.bit_position_opt.is_some() {
            self.engine_status = self.engine_status.clone().set_is_thinking(is_thinking);
        }
    }
    fn is_running(&self) -> bool {
        self.engine_status.is_running()
    }
    fn is_thinking(&self) -> bool {
        self.engine_status.is_thinking()
    }

    fn start_thinking(
        &mut self,
        bit_position: &bitboard::BitPosition,
        game_manager_actor: game_manager::GameManagerActor,
    ) {
        self.game_manager_actor_opt = Some(game_manager_actor);
        assert!(self.is_running() && !self.is_thinking());
        self.bit_position_opt = Some(bit_position.clone());
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDummy of id {:?} started thinking.",
                self.engine.id()
            )));
        }
        self.engine_status = self.engine_status.clone().set_is_thinking(true);
    }
    fn stop_thinking(&mut self) {
        if self.is_thinking() {
            self.set_is_thinking(false);
            self.bit_position_opt = None;
            if let Some(best_move) = &self.ts_best_move_opt {
                let reply = game_manager::SetBestMove::from_ts_move(best_move.clone());
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(format!(
                        "EngineDummy of id {:?} stopped thinking.",
                        self.engine.id()
                    )));
                    debug_actor.do_send(debug::AddMessage(format!(
                        "EngineDummy of id {:?} reply is: '{:?}'",
                        self.engine.id(),
                        reply
                    )));
                }
                self.game_manager_actor_opt.as_ref().unwrap().do_send(reply);
            }
        }
    }
}
impl Actor for EngineDispatcher {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.self_actor_opt = Some(ctx.address());
        self.start_event_loop();
    }
    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        self.stop_event_loop();
    }
}
pub type EngineDispatcherActor = Addr<EngineDispatcher>;

////////////////////////////////////////////////////////

// Implementation
#[derive(Debug, Clone)]
pub struct EngineDummy {
    id_number: String,
    debug_actor_opt: Option<debug::DebugActor>,
}
impl EngineDummy {
    pub fn new(debug_actor_opt: Option<debug::DebugActor>) -> Self {
        Self {
            id_number: "".to_string(),
            debug_actor_opt,
        }
    }
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

impl Engine for EngineDummy {
    fn id(&self) -> EngineId {
        EngineId {
            name: format!("{} {}", DUMMY_ENGINE_ID_NAME.to_owned(), self.id_number)
                .trim()
                .to_string(),
            author: DUMMY_ENGINE_ID_AUTHOR.to_owned(),
        }
    }
    fn think(&self, self_actor: Addr<EngineDispatcher>, bit_position: &bitboard::BitPosition) {
        // first generate moves
        let moves = gen_moves(&bit_position);
        // And then stop thinking and clear positino
        self_actor.do_send(EngineStopThinking);
        let mut rng = ChaCha12Rng::from_entropy();
        let best_move_opt = moves.choose(&mut rng).cloned();
        if let Some(best_move) = best_move_opt {
            let reply = EngineBestMoveFound(best_move);
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "EngineDummy of id {:?} reply is: '{:?}'",
                    self.id(),
                    reply
                )));
            }
            self_actor.do_send(reply);
        } else {
            // FIXME: Do nothing. The engine should be put asleep
            panic!("To be implemented. When EndGame detected in game_manager, stop the engines")
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
#[derive(Debug, Message, Default)]
#[rtype(result = "Option<EngineId>")]
pub struct EngineGetId;
impl Handler<EngineGetId> for EngineDispatcher {
    type Result = Option<EngineId>;

    fn handle(&mut self, msg: EngineGetId, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        Some(self.engine.id())
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EngineGetIdAsync<R>
where
    R: uci::UciRead + 'static,
{
    uci_caller: Addr<uci::UciEntity<R>>,
}
impl<R> EngineGetIdAsync<R>
where
    R: uci::UciRead + 'static,
{
    pub fn new(uci_caller: Addr<uci::UciEntity<R>>) -> Self {
        Self { uci_caller }
    }
}
impl<R> Handler<EngineGetIdAsync<R>> for EngineDispatcher
where
    R: uci::UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: EngineGetIdAsync<R>, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive EngineGetIdAsync",
                self.engine.id()
            )));
        }
        let reply = uci::DisplayEngineId(self.engine.id());
        msg.uci_caller.do_send(reply);
    }
}

#[derive(Debug, Message, Default)]
#[rtype(result = "Option<EngineStatus>")]
pub struct EngineGetStatus;
impl Handler<EngineGetStatus> for EngineDispatcher {
    type Result = Option<EngineStatus>;

    fn handle(&mut self, msg: EngineGetStatus, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        Some(self.engine_status.clone())
    }
}

#[derive(Debug, Message, Default)]
#[rtype(result = "Option<game_manager::TimestampedBitBoardMove>")]
pub struct EngineGetBestMove;
impl Handler<EngineGetBestMove> for EngineDispatcher {
    type Result = Option<game_manager::TimestampedBitBoardMove>;

    fn handle(&mut self, msg: EngineGetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        self.get_best_move()
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineBestMoveFound(bitboard::BitBoardMove);
impl Handler<EngineBestMoveFound> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineBestMoveFound, _ctx: &mut Self::Context) -> Self::Result {
        let forward = super::game_manager::SetBestMove::new(msg.0, self.engine.id());
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} send to game_manager: {:?}",
                self.engine.id(),
                forward
            )));
        }
        self.game_manager_actor_opt
            .as_ref()
            .unwrap()
            .do_send(forward);
        self.set_best_move(Some(msg.0));
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineSetStatus(EngineStatus);
impl Handler<EngineSetStatus> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineSetStatus, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        self.engine_status = msg.0;
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineStartThinking {
    bit_position: bitboard::BitPosition,
    game_manager_actor: game_manager::GameManagerActor,
}
impl EngineStartThinking {
    pub fn new(
        bit_position: bitboard::BitPosition,
        game_manager_actor: game_manager::GameManagerActor,
    ) -> Self {
        EngineStartThinking {
            bit_position,
            game_manager_actor,
        }
    }
}
impl Handler<EngineStartThinking> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineStartThinking, _ctx: &mut Self::Context) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        self.start_thinking(&msg.bit_position, msg.game_manager_actor);
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineStopThinking;

impl Handler<EngineStopThinking> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineStopThinking, _ctx: &mut Self::Context) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        self.stop_thinking();
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineCleanResources;

impl Handler<EngineCleanResources> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineCleanResources, _ctx: &mut Self::Context) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        self.stop_event_loop();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::game::{engine::EngineGetId, game_manager, game_manager::build_game_manager_actor};

    #[actix::test]
    async fn test_engine_dummy() {
        let inputs = vec!["position startpos", "go"];
        let game_manager_actor = build_game_manager_actor(inputs.clone()).await;
        let msg = game_manager::GetCurrentEngine::default();
        let result = game_manager_actor.send(msg).await;
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
        let inputs = vec!["position startpos", "go", "wait100ms"];

        for _ in 0..10 {
            let game_manager_actor = build_game_manager_actor(inputs.clone()).await;
            let ts_best_move = game_manager_actor
                .send(game_manager::GetBestMove)
                .await
                .expect("actix mailbox error") // Ensure no Actix mailbox error
                .expect("No best move found"); // Ensure a best move is found

            let best_move_str = ts_best_move.best_move().cast(); // Convert the best move to the desired format (if necessary)
            best_moves.push(best_move_str); // Add the best move to the Vec
            game_manager_actor
                .send(game_manager::UciCommand::CleanResources)
                .await
                .expect("actix mailbox error")
                .unwrap();
        }
        let unique_moves: HashSet<_> = best_moves.iter().cloned().collect();
        // ensure that we generate random moves
        assert!(unique_moves.len() > 1)
    }
}
