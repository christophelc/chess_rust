use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use actix::{Arbiter, AsyncContext, Handler, Message};

use crate::entity::engine::component::{engine_logic as logic, ts_best_move, ts_bitboard_move};
use crate::entity::game::component::game_state;
use crate::entity::stat::actor::stat_entity;
use crate::ui::notation::long_notation;
use crate::{
    entity::{
        game::{actor::game_manager, component::bitboard},
        uci::actor::uci_entity,
    },
    monitoring::debug,
};

use super::{EngineDispatcher, EngineStatus};

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
pub struct EngineGetIdAsync {
    uci_caller: uci_entity::UciActor,
}
impl EngineGetIdAsync {
    pub fn new(uci_caller: uci_entity::UciActor) -> Self {
        Self { uci_caller }
    }
}
impl Handler<EngineGetIdAsync> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineGetIdAsync, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive EngineGetIdAsync",
                self.engine.id()
            )));
        }
        let reply = uci_entity::handler_uci::DisplayEngineId(self.engine.id());
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
#[rtype(result = "Option<ts_bitboard_move::TimestampedBitBoardMove>")]
pub struct EngineGetBestMove;
impl Handler<EngineGetBestMove> for EngineDispatcher {
    type Result = Option<ts_bitboard_move::TimestampedBitBoardMove>;

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
pub struct EngineEndOfAnalysis(pub bitboard::BitBoardMove);
impl Handler<EngineEndOfAnalysis> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineEndOfAnalysis, _ctx: &mut Self::Context) -> Self::Result {
        tracing::debug!("Received end of analysis");
        self.reset_thinking();
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        // send best move to game manager
        send_best_move(
            msg.0,
            self.engine.id(),
            self.debug_actor_opt.as_ref(),
            self.game_manager_actor_opt.as_ref().unwrap().clone(),
        );
        // display bestmove in uci console
        let ts_best_move = ts_bitboard_move::TimestampedBitBoardMove::new(msg.0, self.engine.id());
        let ts_best_move_cast = ts_best_move::TimestampedBestMove::build(
            long_notation::LongAlgebricNotationMove::build_from_b_move(ts_best_move.best_move()),
            ts_best_move.timestamp(),
            ts_best_move.engine_id(),
        );
        let forward =
            uci_entity::handler_uci::UciResult::DisplayBestMove(Some(ts_best_move_cast), true);
        // send DisplayMove command
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} send to game_manager: {:?}",
                self.engine.id(),
                forward
            )));
        }
        tracing::debug!("uci_caller is {:?}", &self.uci_caller_opt);
        if let Some(uci_caller) = &self.uci_caller_opt {
            tracing::debug!("Sending bestmove to uci");
            uci_caller.do_send(forward);
            self.set_best_move(Some(msg.0));
        } else {
            tracing::debug!("No uci defined. Cannot send best move to uci.");
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineSendBestMove(pub bitboard::BitBoardMove);
impl Handler<EngineSendBestMove> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineSendBestMove, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        send_best_move(
            msg.0,
            self.engine.id(),
            self.debug_actor_opt.as_ref(),
            self.game_manager_actor_opt.as_ref().unwrap().clone(),
        );
        self.set_best_move(Some(msg.0));
    }
}

fn send_best_move(
    best_move: bitboard::BitBoardMove,
    engine_id: logic::EngineId,
    debug_actor_opt: Option<&debug::DebugActor>,
    game_manager: game_manager::GameManagerActor,
) {
    let forward = game_manager::handler_game::SetBestMove::new(best_move, engine_id.clone());
    if let Some(debug_actor) = debug_actor_opt {
        debug_actor.do_send(debug::AddMessage(format!(
            "EngineDispatcher for engine id {:?} send to game_manager: {:?}",
            engine_id, forward
        )));
    }
    game_manager.do_send(forward);
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
pub struct EngineInit;

impl Handler<EngineInit> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, _msg: EngineInit, _ctx: &mut Self::Context) -> Self::Result {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = Arc::clone(&stop_flag);
        // start non blocking task find_best_move
        let self_actor = self.self_actor_opt.as_ref().unwrap().clone();
        let stat_actor_opt = self.stat_actor_opt.as_ref().cloned();
        let game_clone = self.game_opt.as_ref().unwrap().clone();
        let engine = self.engine.clone();
        tracing::debug!("Calling engine");
        actix::Arbiter::spawn(&Arbiter::new(), async move {
            tracing::debug!("Start computing");
            engine.find_best_move(self_actor, stat_actor_opt, game_clone, &stop_flag_clone);
            //thread::sleep(std::time::Duration::from_secs(10));
            tracing::debug!("End computing");
        });
        self.stop_flag = stop_flag;
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct TimeoutCheck {
    timeout: std::time::Duration,
    thinking_id: u64,
}
impl TimeoutCheck {
    pub fn new(timeout: std::time::Duration, thinking_id: u64) -> Self {
        Self {
            timeout,
            thinking_id,
        }
    }
}

impl Handler<TimeoutCheck> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: TimeoutCheck, _ctx: &mut Self::Context) -> Self::Result {
        tracing::debug!("Timeout check triggered");
        let stop_flag = Arc::clone(&self.stop_flag);
        let current_thinking_id = self.thinking_id;
        actix::Arbiter::spawn(&Arbiter::new(), async move {
            tracing::debug!("Sleeping {:?}", msg.timeout);
            tokio::time::sleep(msg.timeout).await;
            tracing::debug!(
                "current thinking_id {} vs previous thinking_id {}",
                current_thinking_id,
                msg.thinking_id
            );
            if msg.thinking_id == current_thinking_id {
                tracing::debug!("Timeout reached: stopping the engine.");
                stop_flag.store(true, Ordering::SeqCst);
            }
        });
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineInitTimeLimit {
    game: game_state::GameState,
}

impl EngineInitTimeLimit {
    pub fn new(game: &game_state::GameState) -> Self {
        Self { game: game.clone() }
    }
}

impl Handler<EngineInitTimeLimit> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineInitTimeLimit, ctx: &mut Self::Context) -> Self::Result {
        tracing::debug!("Initializing engine with time limit");

        // Store game state and setup initial configuration
        self.game_opt = Some(msg.game.clone());
        let game_manager_actor = self.game_manager_actor_opt.as_ref().unwrap().clone();
        let player_turn = msg.game.bit_position().bit_position_status().player_turn();

        // Schedule the timeout check
        let fut = Self::get_max_time_for_move(game_manager_actor, msg.game.clone(), player_turn);
        let ctx_addr = ctx.address();
        let thinking_id = self.thinking_id;
        actix::spawn(async move {
            if let Some(max_time) = fut.await {
                tracing::debug!("Scheduling timeout for {} seconds", max_time.as_secs());
                ctx_addr.do_send(TimeoutCheck::new(max_time, thinking_id));
            } else {
                tracing::warn!("Could not determine max time for move.");
            }
        });
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineStartThinking {
    game: game_state::GameState,
    game_manager_actor: game_manager::GameManagerActor,
    uci_caller: uci_entity::UciActor,
    stat_actor_opt: Option<stat_entity::StatActor>,
}
impl EngineStartThinking {
    pub fn new(
        game: game_state::GameState,
        game_manager_actor: game_manager::GameManagerActor,
        uci_caller: uci_entity::UciActor,
        stat_actor_opt: Option<stat_entity::StatActor>,
    ) -> Self {
        EngineStartThinking {
            game,
            game_manager_actor,
            uci_caller,
            stat_actor_opt,
        }
    }
}
impl Handler<EngineStartThinking> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineStartThinking, _ctx: &mut Self::Context) {
        self.uci_caller_opt = Some(msg.uci_caller.clone());
        self.stat_actor_opt = msg.stat_actor_opt.clone();
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        if let Some(stat_actor) = msg.stat_actor_opt {
            let msg = stat_entity::handler_stat::StatInit(self.engine.id());
            stat_actor.do_send(msg.clone());
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "EngineDispatcher for engine id {:?} receive {:?}",
                    self.engine.id(),
                    msg
                )));
            }
        }
        self.start_thinking(&msg.game, msg.game_manager_actor);
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct EngineStopThinking(Option<stat_entity::StatActor>);
impl EngineStopThinking {
    pub fn new(stat_actor_opt: Option<stat_entity::StatActor>) -> Self {
        Self(stat_actor_opt)
    }
}

impl Handler<EngineStopThinking> for EngineDispatcher {
    type Result = ();

    fn handle(&mut self, msg: EngineStopThinking, _ctx: &mut Self::Context) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            tracing::debug!("Receive EngineStopThinking");
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive {:?}",
                self.engine.id(),
                msg
            )));
        }
        let stat_actor_opt = msg.0;
        if let Some(stat_actor) = stat_actor_opt {
            let msg = stat_entity::handler_stat::StatClose::new(self.engine.id());
            stat_actor.do_send(msg.clone());
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!(
                    "EngineDispatcher for engine id {:?} send {:?}",
                    self.engine.id(),
                    msg
                )));
            }
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
        self.reset_thinking();
    }
}
