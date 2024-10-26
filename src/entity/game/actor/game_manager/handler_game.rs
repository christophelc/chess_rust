use actix::{ActorContext, Handler, Message};

use crate::{
    entity::{
        engine::component::{ts_best_move, ts_bitboard_move}, game::component::{bitboard, game_state, parameters}, uci::actor::uci_entity
    },
    monitoring::debug,
    ui::notation::long_notation,
};

use super::GameManager;
use crate::entity::engine::component::engine_logic as logic;

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct GetBestMoveFromUci {
    uci_caller: uci_entity::UciActor,
}
impl GetBestMoveFromUci {
    pub fn new(uci_caller: uci_entity::UciActor) -> Self {
        Self { uci_caller }
    }
}

impl Handler<GetBestMoveFromUci> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: GetBestMoveFromUci, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(
                "game_manager_actor receive GetBestMoveFromUci".to_string(),
            ));
        }
        let engine_still_thinking = false;
        let reply = uci_entity::handler_uci::UciResult::DisplayBestMove(
            self.ts_best_move_opt.clone(),
            !engine_still_thinking,
        );
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor send to uci entity: '{:?}'",
                reply
            )));
        }
        msg.uci_caller.do_send(reply);
    }
}

pub struct SendBestMoveToUci;

#[derive(Debug, Message)]
#[rtype(result = "Option<ts_best_move::TimestampedBestMove>")]
pub struct GetBestMove;

impl Handler<GetBestMove> for GameManager {
    type Result = Option<ts_best_move::TimestampedBestMove>;

    fn handle(&mut self, msg: GetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        self.ts_best_move_opt.clone()
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<game_state::EndGame, ()>")]
pub struct GetEndGame;

impl Handler<GetEndGame> for GameManager {
    type Result = Result<game_state::EndGame, ()>;

    fn handle(&mut self, msg: GetEndGame, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        let end_game = match &self.game_state_opt {
            None => game_state::EndGame::None,
            Some(game_state) => game_state.end_game(),
        };
        Ok(end_game)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Option<game_state::GameState>")]
pub struct GetGameState;

impl Handler<GetGameState> for GameManager {
    type Result = Option<game_state::GameState>;

    fn handle(&mut self, msg: GetGameState, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        self.game_state().cloned()
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<parameters::Parameters, ()>")]
pub struct GetParameters;

impl Handler<GetParameters> for GameManager {
    type Result = Result<parameters::Parameters, ()>;

    fn handle(&mut self, msg: GetParameters, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        Ok(self.parameters.clone())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetBestMove(pub ts_bitboard_move::TimestampedBitBoardMove);
impl SetBestMove {
    pub fn new(best_move: bitboard::BitBoardMove, engine_id: logic::EngineId) -> Self {
        Self(ts_bitboard_move::TimestampedBitBoardMove::new(best_move, engine_id))
    }
    pub fn from_ts_move(ts_move: ts_bitboard_move::TimestampedBitBoardMove) -> Self {
        Self(ts_move)
    }
}

impl Handler<SetBestMove> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: SetBestMove, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        let ts_best_move = msg.0;
        let ts_best_move_cast = ts_best_move::TimestampedBestMove::build(
            long_notation::LongAlgebricNotationMove::build_from_b_move(ts_best_move.best_move()),
            ts_best_move.timestamp(),
            ts_best_move.engine_id(),
        );
        let mut is_update = true;
        if let Some(ts_best_move) = &self.ts_best_move_opt {
            if ts_best_move.is_more_recent_best_move_than(&ts_best_move_cast) {
                is_update = false;
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(
                        "best move not updated because not more recent than the current one"
                            .to_string(),
                    ));
                }
            }
        }
        if is_update {
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage("best move updated".to_string()));
            }
            self.ts_best_move_opt = Some(ts_best_move_cast);
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<(), String>")]
pub struct PlayMoves {
    moves: Vec<long_notation::LongAlgebricNotationMove>,
}
impl PlayMoves {
    pub fn new(moves: Vec<long_notation::LongAlgebricNotationMove>) -> Self {
        Self { moves }
    }
}

impl Handler<PlayMoves> for GameManager {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: PlayMoves, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        self.play_moves(msg.moves)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Result<super::History, ()>")]
pub struct GetHistory;

impl Handler<GetHistory> for GameManager {
    type Result = Result<super::History, ()>;

    fn handle(&mut self, msg: GetHistory, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        Ok(self.history().clone())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct StopActor;

impl Handler<StopActor> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: StopActor, ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        ctx.stop();
    }
}
