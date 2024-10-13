pub mod handler_engine;

use std::sync::Arc;

use actix::prelude::*;
use tokio::task::{spawn_local, JoinHandle};

use crate::entity::engine::component::engine_logic as logic;
use crate::entity::game::actor::game_manager;
use crate::entity::game::component::bitboard::{self, BitBoardMove};
use crate::monitoring::debug;

pub struct EngineDispatcher {
    engine: Arc<dyn logic::Engine + Send + Sync>, // EngineActor dans un Arc
    debug_actor_opt: Option<debug::DebugActor>,
    engine_status: EngineStatus,
    ts_best_move_opt: Option<game_manager::handler_game::TimestampedBitBoardMove>,
    self_actor_opt: Option<Addr<EngineDispatcher>>,
    game_manager_actor_opt: Option<game_manager::GameManagerActor>,
    bit_position_opt: Option<bitboard::BitPosition>, // initial position to be played
    thread_find_best_move_opt: Option<JoinHandle<()>>,
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
            thread_find_best_move_opt: None,
        }
    }
    fn get_best_move(&self) -> Option<game_manager::handler_game::TimestampedBitBoardMove> {
        self.ts_best_move_opt.clone()
    }
    fn set_best_move(&mut self, best_move_opt: Option<BitBoardMove>) {
        self.ts_best_move_opt = best_move_opt.map(|best_move| {
            game_manager::handler_game::TimestampedBitBoardMove::new(best_move, self.engine.id())
        });
    }
    fn set_is_thinking(&mut self, is_thinking: bool) {
        if self.bit_position_opt.is_some() {
            self.engine_status = self.engine_status.clone().set_is_thinking(is_thinking);
        }
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
        assert!(!self.is_thinking());
        self.bit_position_opt = Some(bit_position.clone());
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDummy of id {:?} started thinking.",
                self.engine.id()
            )));
        }
        self.engine_status = self.engine_status.clone().set_is_thinking(true);

        // start non blocking task find_best_move
        let self_actor = self.self_actor_opt.as_ref().unwrap().clone();
        let bit_position = bit_position.clone();
        let engine = self.engine.clone();
        let thread_find_best_move = spawn_local(async move {
            engine.find_best_move(self_actor, bit_position);
        });
        self.thread_find_best_move_opt = Some(thread_find_best_move);
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "Start find best move for Engine id {:?}.",
                self.engine.id()
            )))
        }
    }
    fn reset_thinking(&mut self) {
        self.set_is_thinking(false);
        if let Some(thread) = &self.thread_find_best_move_opt {
            thread.abort();
        }
    }
    fn stop_thinking(&mut self) {
        if self.is_thinking() {
            self.reset_thinking();
            self.bit_position_opt = None;
            if let Some(best_move) = &self.ts_best_move_opt {
                let reply =
                    game_manager::handler_game::SetBestMove::from_ts_move(best_move.clone());
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
    }
    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        self.reset_thinking();
    }
}
pub type EngineDispatcherActor = Addr<EngineDispatcher>;
#[derive(Debug, Default, PartialEq, Clone)]
pub struct EngineStatus {
    is_thinking: bool, // thinking
}
impl EngineStatus {
    #[cfg(test)]
    pub fn new(is_thinking: bool) -> Self {
        Self { is_thinking }
    }
    pub fn is_thinking(&self) -> bool {
        self.is_thinking
    }
    pub fn set_is_thinking(&self, is_thinking: bool) -> Self {
        Self { is_thinking }
    }
}
