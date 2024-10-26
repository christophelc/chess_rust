use std::io::Write;

use actix::{dev::ContextFutureSpawner, AsyncContext, Handler, Message, WrapFuture};

use crate::{
    entity::{game::actor::game_manager, uci::component::event},
    monitoring::debug,
    ui::notation::{
        fen::{self, EncodeUserInput},
        long_notation,
    },
};

use super::{
    handler_poll, handler_uci, HandleEventError, StatePollingUciEntity, UciEntity,
};

#[derive(Message)]
#[rtype(result = "()")]
pub struct ProcessEvents(pub Vec<event::Event>);

impl Handler<ProcessEvents> for UciEntity {
    type Result = ();

    fn handle(&mut self, msg: ProcessEvents, ctx: &mut Self::Context) -> Self::Result {
        let events = msg.0;

        let addr = ctx.address();

        // Spawn a future within the actor context
        async move {
            for event in events {
                // Send the event and await its result
                let result = addr.send(event).await;

                match result {
                    Ok(_) => {
                        // Handle successful result
                    }
                    Err(e) => {
                        // Handle error
                        println!("Failed to send event: {:?}", e);
                    }
                }
            }
        }
        .into_actor(self) // Converts the future to an Actix-compatible future
        .spawn(ctx); // Spawns the future in the actor's context
    }
}

impl Handler<event::Event> for UciEntity {
    type Result = ();

    fn handle(&mut self, msg: event::Event, ctx: &mut Self::Context) -> Self::Result {
        let actor_self = ctx.address();
        match msg {
            event::Event::Write(s) => {
                writeln!(self.stdout, "{}", s).unwrap();
                self.stdout.flush().unwrap();
            }
            event::Event::WriteDebug(s) => {
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(format!("Uci actor event: {}", s)))
                }
            }
            event::Event::DebugMode(debug_actor_opt) => {
                self.debug_actor_opt = debug_actor_opt;
            }
            event::Event::StartPos => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::InitPosition);
            }
            event::Event::Fen(fen) => {
                let position = fen::Fen::decode(&fen).expect("Failed to decode FEN");
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::UpdatePosition(
                        fen.to_string(),
                        position,
                    ),
                );
            }
            // Go command
            event::Event::Depth(depth) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::DepthFinite(depth),
                );
            }
            event::Event::SearchInfinite => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::SearchInfinite);
            }
            event::Event::MaxTimePerMoveInMs(time) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::MaxTimePerMoveInMs(time),
                );
            }
            ref event @ event::Event::Moves(ref moves) => match moves_validation(moves) {
                Ok(valid_moves) => {
                    self.game_manager_actor.do_send(
                        game_manager::handler_uci_command::UciCommand::ValidMoves {
                            moves: valid_moves,
                        },
                    );
                }
                Err(err) => {
                    actor_self.do_send(handler_uci::UciResult::Err(HandleEventError::new(
                        event.clone(),
                        err,
                    )));
                }
            },
            event::Event::Wtime(wtime) => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::Wtime(wtime));
            }
            event::Event::Btime(btime) => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::Btime(btime));
            }
            event::Event::WtimeInc(wtime_inc) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::WtimeInc(wtime_inc),
                );
            }
            event::Event::BtimeInc(btime_inc) => {
                self.game_manager_actor.do_send(
                    game_manager::handler_uci_command::UciCommand::BtimeInc(btime_inc),
                );
            }
            ref event @ event::Event::SearchMoves(ref search_moves) => {
                match moves_validation(search_moves) {
                    Ok(valid_moves) => {
                        self.game_manager_actor.do_send(
                            game_manager::handler_uci_command::UciCommand::SearchMoves(valid_moves),
                        );
                    }
                    Err(err) => {
                        actor_self.do_send(handler_uci::UciResult::Err(HandleEventError::new(
                            event.clone(),
                            err,
                        )));
                    }
                }
            }
            event::Event::StartEngineThinking => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::EngineStartThinking);
                self.state_polling = StatePollingUciEntity::Polling;
                ctx.address().do_send(handler_poll::PollBestMove);
            }
            event::Event::StopEngine => {
                self.game_manager_actor
                    .do_send(game_manager::handler_uci_command::UciCommand::EngineStopThinking);
            }
            event::Event::Quit => {
                actor_self.do_send(handler_uci::UciResult::Quit);
            }
        }
    }
}

pub fn moves_validation(
    moves: &Vec<String>,
) -> Result<Vec<long_notation::LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<long_notation::LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match long_notation::LongAlgebricNotationMove::build_from_str(m) {
            Ok(valid_move) => valid_moves.push(valid_move),
            Err(err) => errors.push(err),
        }
    }
    if !errors.is_empty() {
        Err(errors.join(", "))
    } else {
        Ok(valid_moves)
    }
}
