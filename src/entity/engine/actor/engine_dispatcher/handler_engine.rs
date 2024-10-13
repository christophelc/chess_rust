use actix::{Addr, Handler, Message};

use crate::entity::engine::component::engine_logic as logic;
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
pub struct EngineGetIdAsync<R>
where
    R: uci_entity::UciRead + 'static,
{
    uci_caller: Addr<uci_entity::UciEntity<R>>,
}
impl<R> EngineGetIdAsync<R>
where
    R: uci_entity::UciRead + 'static,
{
    pub fn new(uci_caller: Addr<uci_entity::UciEntity<R>>) -> Self {
        Self { uci_caller }
    }
}
impl<R> Handler<EngineGetIdAsync<R>> for EngineDispatcher
where
    R: uci_entity::UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: EngineGetIdAsync<R>, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "EngineDispatcher for engine id {:?} receive EngineGetIdAsync",
                self.engine.id()
            )));
        }
        let reply = uci_entity::DisplayEngineId(self.engine.id());
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
#[rtype(result = "Option<game_manager::handler_game::TimestampedBitBoardMove>")]
pub struct EngineGetBestMove;
impl Handler<EngineGetBestMove> for EngineDispatcher {
    type Result = Option<game_manager::handler_game::TimestampedBitBoardMove>;

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
        let forward = game_manager::handler_game::SetBestMove::new(msg.0, self.engine.id());
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
