use actix::{ActorContext, Handler, Message};

use crate::{entity::game::actor::game_manager, monitoring::debug};

use super::{StatePollingUciEntity, UciEntity, UciRead};

use std::io::Write;

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub enum UciResult {
    Quit,
    DisplayBestMove(Option<game_manager::TimestampedBestMove>, bool), // maybe move, display in uci ui 'bestmove ...': bool
    Err(super::HandleEventError),
}

impl<R: UciRead> Handler<UciResult> for UciEntity<R> {
    type Result = ();

    fn handle(&mut self, msg: UciResult, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            UciResult::DisplayBestMove(timestamped_best_move_opt, is_show) => {
                if let Some(timestamped_best_move) = timestamped_best_move_opt {
                    // TODO: compare best move timestamp ? We could imagine competition between engine of different type searching for the best move
                    let msg_best_move =
                        format!("bestmove {}", timestamped_best_move.best_move().cast());
                    let msg_ts = format!("timestamp: {}", timestamped_best_move.timestamp());
                    let msg_origin = format!("origin: {:?}", timestamped_best_move.origin());
                    let msg = [msg_best_move, msg_ts, msg_origin].join(", ");
                    if let Some(debug_actor) = &self.debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(msg.to_string()));
                    }
                    if is_show {
                        let _ = writeln!(self.stdout, "{}", msg);
                        self.stdout.flush().unwrap();
                    }
                }
                self.state_polling = StatePollingUciEntity::Pending;
            }
            UciResult::Err(err) => {
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(err.to_string()));
                }
            }
            UciResult::Quit => {
                self.game_manager_actor
                    .do_send(game_manager::handler_game::StopActor);
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(
                        "Send Stop message to game manager.".to_string(),
                    ))
                }
                ctx.stop();
            }
        }
    }
}
