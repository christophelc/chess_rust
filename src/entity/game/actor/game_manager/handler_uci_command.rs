use actix::{
    dev::ContextFutureSpawner, ActorFutureExt, AsyncContext, Handler, Message, WrapFuture,
};

use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::square::Switch;
use crate::{
    entity::{clock::actor::chessclock, game::component::game_state},
    monitoring::debug,
    ui::notation::{fen, long_notation},
};

use super::{handler_game::TimestampedBitBoardMove, GameManager};

#[derive(Debug, Message)]
#[rtype(result = "Result<(), String>")]
pub enum UciCommand {
    Btime(u64),                                                // Update clock for black
    BtimeInc(u64),                                             // Update increment clock for black
    CleanResources,                                            // Clean resources
    DepthFinite(u32),                                          // Set depth
    EngineStartThinking,                                       // Go command: start calculating
    EngineStopThinking,                                        // Stop command: retrieve best move
    InitPosition,                                              // Set starting position
    MaxTimePerMoveInMs(u32),                                   // Set maximum time per move
    SearchMoves(Vec<long_notation::LongAlgebricNotationMove>), // Focus on a list of moves for analysis
    SearchInfinite,                                            // Set infinite search
    UpdatePosition(String, fen::Position),                     // Set a new position
    ValidMoves {
        // Play moves from the current position
        moves: Vec<long_notation::LongAlgebricNotationMove>,
    },
    Wtime(u64),    // Update clock for white
    WtimeInc(u64), // Update increment clock for white
}

impl Handler<UciCommand> for GameManager {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: UciCommand, ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive command: {:?}",
                msg
            )));
        }
        let mut result = Ok(());
        match msg {
            UciCommand::Btime(time) => {
                match &self.black_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(clock_actor) => {
                        clock_actor.do_send(chessclock::handler_clock::SetRemainingTime::new(time));
                    }
                }
                // for the moment, we memorize the inital parameters
                self.parameters.set_btime(time);
            }
            UciCommand::BtimeInc(time_inc) => {
                match &self.black_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(clock_actor) => {
                        clock_actor.do_send(chessclock::handler_clock::SetIncTime::new(time_inc));
                    }
                }
                // for the moment, we memorize the inital parameters
                self.parameters.set_btime_inc(time_inc);
            }
            UciCommand::InitPosition => {
                let position = fen::Position::build_initial_position();
                self.game_state_opt =
                    Some(game_state::GameState::new(position, &self.zobrist_table));
                self.history.init();
                self.ts_best_move_opt = None;
            }
            UciCommand::Wtime(time) => {
                match &self.white_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(white_clock_actor) => {
                        white_clock_actor
                            .do_send(chessclock::handler_clock::SetRemainingTime::new(time));
                    }
                }
                // for the moment, we memorize the inital parameters
                self.parameters.set_wtime(time);
            }
            UciCommand::WtimeInc(time_inc) => {
                match &self.white_clock_actor_opt {
                    None => {
                        // do nothing
                    }
                    Some(white_clock_actor) => {
                        white_clock_actor
                            .do_send(chessclock::handler_clock::SetIncTime::new(time_inc));
                    }
                }
                // for the moment, we memorize the inital parameters
                self.parameters.set_wtime_inc(time_inc);
            }
            UciCommand::DepthFinite(depth) => {
                self.parameters.set_depth(depth);
            }
            UciCommand::SearchInfinite => {
                self.parameters.set_depth_infinite();
            }
            UciCommand::MaxTimePerMoveInMs(time) => {
                self.parameters.set_time_per_move_in_ms(time);
            }
            UciCommand::UpdatePosition(fen, position) => {
                self.game_state_opt =
                    Some(game_state::GameState::new(position, &self.zobrist_table));
                self.history.set_fen(&fen);
                self.ts_best_move_opt = None;
                if let Some(debug_actor) = &self.debug_actor_opt {
                    let msg = format!(
                        "New position is:\n{}",
                        self.game_state_opt
                            .as_ref()
                            .unwrap()
                            .bit_position()
                            .to()
                            .chessboard()
                    );
                    debug_actor.do_send(debug::AddMessage(msg));
                }
            }
            UciCommand::SearchMoves(search_moves) => {
                self.parameters.set_search_moves(search_moves);
            }
            UciCommand::ValidMoves { moves } => {
                result = self.play_moves(moves);
            }
            UciCommand::EngineStartThinking => {
                if let Some(ref game_state) = &self.game_state_opt {
                    let color = game_state
                        .bit_position()
                        .bit_position_status()
                        .player_turn();
                    let engine_actor_or_error = self.players.get_engine(color);
                    match engine_actor_or_error {
                        Ok(engine_actor) => {
                            let msg = dispatcher::handler_engine::EngineStartThinking::new(
                                game_state.clone(),
                                ctx.address().clone(),
                            );
                            if let Some(debug_actor) = &self.debug_actor_opt {
                                debug_actor.do_send(debug::AddMessage(format!(
                                    "game_manager_actor forward message to engine_actor for color {:?}: {:?}", color,
                                    msg
                                )));
                            }
                            engine_actor.do_send(msg);
                        }
                        Err(err) => result = Err(err),
                    }
                }
            }
            UciCommand::CleanResources => {
                if let Some(game_state) = &self.game_state_opt {
                    // clean resources for each engine actor
                    let color = game_state
                        .bit_position()
                        .bit_position_status()
                        .player_turn();
                    let engine_current_actor = self.players.get_engine(color).ok();
                    let engine_opponent_actor = self.players.get_engine(color.switch()).ok();
                    if let Some(debug_actor) = &self.debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(
                            "game_manager_actor forward message to engines_actor: EngineCleanResources".to_string()));
                    }
                    for engine_actor in engine_current_actor
                        .iter()
                        .chain(engine_opponent_actor.iter())
                    {
                        engine_actor
                            .send(dispatcher::handler_engine::EngineCleanResources)
                            .into_actor(self)
                            .map(|_result, _act, _ctx| ())
                            .wait(ctx);
                    }
                }
            }
            UciCommand::EngineStopThinking => match &self.game_state_opt {
                None => {
                    self.ts_best_move_opt = None;
                    result =
                        Err("No bestmove since no valid position has been entered.".to_string());
                }
                Some(game_state) => {
                    match self.players.get_engine(
                        game_state
                            .bit_position()
                            .bit_position_status()
                            .player_turn(),
                    ) {
                        Ok(engine_actor) => {
                            // stop thinking
                            engine_actor.do_send(dispatcher::handler_engine::EngineStopThinking);
                            let engine_msg = dispatcher::handler_engine::EngineGetBestMove;
                            let debug_actor_opt = self.debug_actor_opt.clone();
                            engine_actor
                                .send(engine_msg)
                                .into_actor(self)
                                .map(
                                    move |result: Result<Option<TimestampedBitBoardMove>, _>,
                                          act,
                                          _ctx| {
                                        match result {
                                            Ok(Some(best_move)) => {
                                                if let Some(debug_actor) = &debug_actor_opt {
                                                    debug_actor.do_send(debug::AddMessage(
                                                        "Best move updated successfully"
                                                            .to_string(),
                                                    ));
                                                }
                                                act.ts_best_move_opt =
                                                    Some(best_move.to_ts_best_move());
                                            }
                                            Ok(None) => {
                                                if let Some(debug_actor) = &debug_actor_opt {
                                                    debug_actor.do_send(debug::AddMessage(
                                                        "No move found.".to_string(),
                                                    ));
                                                }
                                                act.ts_best_move_opt = None;
                                            }
                                            Err(e) => {
                                                if let Some(debug_actor) = &debug_actor_opt {
                                                    debug_actor.do_send(debug::AddMessage(
                                                        format!(
                                                            "Error sending message to engine: {:?}",
                                                            e
                                                        ),
                                                    ));
                                                }
                                                act.ts_best_move_opt = None;
                                            }
                                        }
                                    },
                                )
                                .wait(ctx); // Wait for the future to complete within the actor context
                        }
                        Err(err) => {
                            println!("Failed to retrieve engine actor: {:?}", err);
                            self.ts_best_move_opt = None;
                        }
                    }
                } // Stop engine search
            },
        }
        result
    }
}