use crate::entity::{
    game::actor::game_manager,
    uci::component::{command, event},
};
use actix::{dev::ContextFutureSpawner, AsyncContext, Handler, WrapFuture};

use super::{handler_event, UciEntity, UciRead};

impl<R> Handler<command::Command> for UciEntity<R>
where
    R: UciRead + 'static,
{
    type Result = ();

    fn handle(&mut self, msg: command::Command, ctx: &mut Self::Context) -> Self::Result {
        let mut events: Vec<event::Event> = vec![];
        match msg {
            command::Command::Wait100ms => {
                use tokio::time::{sleep, Duration};

                events.push(event::Event::WriteDebug("waiting 100ms".to_string()));
                sleep(Duration::from_millis(100)).into_actor(self).wait(ctx);
            }
            command::Command::Uci(game_manager_actor) => {
                let uci_caller = ctx.address();
                let msg = game_manager::handler_engine::GetCurrentEngineAsync::new(uci_caller);
                game_manager_actor.do_send(msg);
            }
            command::Command::Ignore => {}
            command::Command::IsReady => events.push(event::Event::Write("readyok".to_string())),
            command::Command::DebugMode(is_debug) => {
                events.push(event::Event::WriteDebug(format!(
                    "debug mode set to {}",
                    is_debug
                )));
                events.push(event::Event::DebugMode(self.debug_actor_opt.clone()));
            }
            command::Command::NewGame => {
                events.push(event::Event::StartPos);
                // TODO: reset btime, wtime ?
            }
            command::Command::Position(pos) => {
                events.push(event::Event::WriteDebug("Position received".to_string()));
                if pos.startpos() {
                    events.push(event::Event::WriteDebug(
                        "Set board to starting position.".to_string(),
                    ));
                    events.push(event::Event::StartPos);
                } else if let Some(fen_str) = pos.fen() {
                    events.push(event::Event::WriteDebug(
                        format!("Set board to FEN: {}", fen_str).to_string(),
                    ));
                    events.push(event::Event::Fen(fen_str));
                }
                if !pos.moves().is_empty() {
                    events.push(event::Event::WriteDebug(
                        format!("Moves played: {:?}", pos.moves()).to_string(),
                    ));
                    events.push(event::Event::Moves(pos.moves().clone()));
                }
            }
            command::Command::Go(go) => {
                if let Some(d) = go.depth() {
                    events.push(event::Event::WriteDebug(
                        format!("Searching to depth: {}", d).to_string(),
                    ));
                    events.push(event::Event::Depth(d));
                }
                if let Some(time) = go.movetime() {
                    events.push(event::Event::WriteDebug(
                        format!("Max time for move: {} ms", time).to_string(),
                    ));
                    events.push(event::Event::MaxTimePerMoveInMs(time));
                }
                if go.infinite() {
                    events.push(event::Event::WriteDebug(
                        "Searching indefinitely...".to_string(),
                    ));
                    events.push(event::Event::SearchInfinite);
                }
                if let Some(wtime) = go.wtime() {
                    events.push(event::Event::WriteDebug(
                        format!("White time left: {} ms", wtime).to_string(),
                    ));
                    events.push(event::Event::Wtime(wtime));
                }
                if let Some(btime) = go.btime() {
                    events.push(event::Event::WriteDebug(
                        format!("Black time left: {} ms", btime).to_string(),
                    ));
                    events.push(event::Event::Btime(btime));
                }
                if let Some(wtime_inc) = go.wtime_inc() {
                    events.push(event::Event::WriteDebug(
                        format!("White time inc: {} ms", wtime_inc).to_string(),
                    ));
                    events.push(event::Event::WtimeInc(wtime_inc));
                }
                if let Some(btime_inc) = go.btime_inc() {
                    events.push(event::Event::WriteDebug(
                        format!("Black time left: {} ms", btime_inc).to_string(),
                    ));
                    events.push(event::Event::BtimeInc(btime_inc));
                }
                if !go.search_moves().is_empty() {
                    events.push(event::Event::WriteDebug(format!(
                        "Limit search to these moves: {:?}",
                        go.search_moves()
                    )));
                    events.push(event::Event::SearchMoves(go.search_moves().clone()));
                }
                events.push(event::Event::StartEngine)
            }
            command::Command::Stop => {
                events.push(event::Event::WriteDebug("Stopping search.".to_string()));
                events.push(event::Event::StopEngine);
            }
            command::Command::Quit => {
                events.push(event::Event::WriteDebug(
                    "Stopping search (Quit).".to_string(),
                ));
                events.push(event::Event::StopEngine);
                events.push(event::Event::WriteDebug("Exiting engine".to_string()));
                events.push(event::Event::Quit);
            }
        }
        let msg = handler_event::ProcessEvents(events);
        ctx.address().do_send(msg);
    }
}
