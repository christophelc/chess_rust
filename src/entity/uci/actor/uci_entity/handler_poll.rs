use std::time::Duration;

use crate::{
    entity::game::actor::game_manager::{self, handler_game},
    monitoring::debug,
};
use actix::{AsyncContext, Handler, Message};

use super::{StatePollingUciEntity, UciEntity, UciRead};

#[derive(Message)]
#[rtype(result = "()")]
pub struct PollBestMove;

// TODO: Uci is in charge of polling game_engine_actor for best move each 100ms
// Handle polling requests from UCI actor
impl<R> Handler<PollBestMove> for UciEntity<R>
where
    R: UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, _msg: PollBestMove, ctx: &mut Self::Context) -> Self::Result {
        self.game_manager_actor
            .do_send(game_manager::handler_game::GetBestMoveFromUci::new(
                ctx.address(),
            ));
        let debug_actor_opt = self.debug_actor_opt.clone();
        if self.state_polling == StatePollingUciEntity::Polling {
            ctx.run_later(
                Duration::from_millis(super::POLLING_INTERVAL_MS),
                move |actor, ctx| {
                    if let Some(debug_actor) = debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(
                            "UciEntity polling Game Manager to get best move...".to_string(),
                        ));
                    }
                    actor
                        .game_manager_actor
                        .do_send(handler_game::GetBestMoveFromUci::new(ctx.address().clone()));
                },
            );
        }
    }
}
