use actix::{Handler, Message};

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
        if let Some(uci_caller) = &self.uci_caller_opt {
            uci_caller.do_send(forward);
            self.set_best_move(Some(msg.0));
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
