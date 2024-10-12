use actix::Message;
use std::sync::Arc;

use tokio::task::spawn_local;
use tokio::time::{sleep, Duration};

use actix::prelude::*;

use crate::entity::engine::component::engine_logic as logic;
use crate::entity::game::actor::game_manager;
use crate::entity::game::component::bitboard::{self, BitBoardMove};
use crate::monitoring::debug;
use crate::ui::uci;

pub struct EngineDispatcher {
    engine: Arc<dyn logic::Engine + Send + Sync>, // EngineActor dans un Arc
    debug_actor_opt: Option<debug::DebugActor>,
    engine_status: EngineStatus,
    ts_best_move_opt: Option<game_manager::TimestampedBitBoardMove>,
    self_actor_opt: Option<Addr<EngineDispatcher>>,
    game_manager_actor_opt: Option<game_manager::GameManagerActor>,
    bit_position_opt: Option<bitboard::BitPosition>, // initial position to be played
}
impl EngineDispatcher {
    pub fn new(
        engine: Arc<dyn logic::Engine + Send + Sync>,
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
#[rtype(result = "Option<logic::EngineId>")]
pub struct EngineGetId;
impl Handler<EngineGetId> for EngineDispatcher {
    type Result = Option<logic::EngineId>;

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
pub struct EngineBestMoveFound(pub bitboard::BitBoardMove);
impl Handler<EngineBestMoveFound> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineBestMoveFound, _ctx: &mut Self::Context) -> Self::Result {
        let forward = game_manager::SetBestMove::new(msg.0, self.engine.id());
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
