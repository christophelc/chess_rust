use actix::{ActorContext, Handler, Message};

use crate::{
    entity::{engine::component::ts_best_move, game::actor::game_manager},
    monitoring::debug,
};

use super::UciEntity;
use crate::entity::engine::component::engine_logic as logic;

use std::io::{self, Write};

#[derive(Message)]
#[rtype(result = "Result<(), io::Error>")]
pub struct DisplayEngineId(pub logic::EngineId);

impl Handler<DisplayEngineId> for UciEntity {
    type Result = Result<(), io::Error>;

    fn handle(&mut self, msg: DisplayEngineId, _ctx: &mut Self::Context) -> Self::Result {
        let engine_id = msg.0;
        writeln!(self.stdout, "id name {}", engine_id.name())?;
        writeln!(self.stdout, "id author {}", engine_id.author())?;
        writeln!(self.stdout, "uciok")?;
        Ok(())
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub enum UciResult {
    Quit,
    DisplayBestMove(Option<ts_best_move::TimestampedBestMove>, bool), // maybe move, display in uci ui 'bestmove ...': bool
    Err(super::HandleEventError),
}

impl Handler<UciResult> for UciEntity {
    type Result = ();

    fn handle(&mut self, msg: UciResult, ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!("uci_actor receives {:?}", msg)));
        }
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
                        tracing::debug!("writing bestmove");
                        let mut handle = std::io::BufWriter::new(io::stdout());
                        //let mut handle = self.stdout.lock();
                        writeln!(handle, "{}", msg).unwrap(); // Write message with a newline
                        handle.flush().unwrap();
                        tracing::debug!("done");
                    }
                }
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
