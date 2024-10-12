use actix::{AsyncContext, Handler, Message};
use std::error::Error;
use std::io::Write;

use crate::entity::game::actor::game_manager;
use crate::monitoring::debug;
use crate::ui::notation::fen::{self, EncodeUserInput};
use crate::ui::notation::long_notation;
use crate::ui::uci;

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub enum Event {
    Btime(u64),
    BtimeInc(u64),
    DebugMode(Option<debug::DebugActor>),
    Depth(u32),
    Fen(String),
    StartEngine,
    Moves(Vec<String>),
    Quit,
    SearchMoves(Vec<String>),
    SearchInfinite,
    StartPos,
    StopEngine,
    MaxTimePerMoveInMs(u32),
    Write(String),
    WriteDebug(String),
    Wtime(u64),
    WtimeInc(u64),
}

#[derive(Debug)]
pub struct HandleEventError {
    event: Event,
    error: String,
}
impl HandleEventError {
    pub fn new(event: Event, error: String) -> Self {
        HandleEventError { event, error }
    }
}
impl Error for HandleEventError {}
impl std::fmt::Display for HandleEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "HandleEvent error for event {:?}. The error is: {}",
            self.event, self.error
        )
    }
}
impl<R: uci::UciRead> Handler<Event> for uci::UciEntity<R> {
    type Result = ();

    fn handle(&mut self, msg: Event, ctx: &mut Self::Context) -> Self::Result {
        let actor_self = ctx.address();
        match msg {
            Event::Write(s) => {
                writeln!(self.stdout, "{}", s).unwrap();
                self.stdout.flush().unwrap();
            }
            Event::WriteDebug(s) => {
                if let Some(debug_actor) = &self.debug_actor_opt {
                    debug_actor.do_send(debug::AddMessage(format!("Uci actor event: {}", s)))
                }
            }
            Event::DebugMode(debug_actor_opt) => {
                self.debug_actor_opt = debug_actor_opt;
            }
            Event::StartPos => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::InitPosition);
            }
            Event::Fen(fen) => {
                let position = fen::Fen::decode(&fen).expect("Failed to decode FEN");
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::UpdatePosition(
                        fen.to_string(),
                        position,
                    ));
            }
            // Go command
            Event::Depth(depth) => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::DepthFinite(depth));
            }
            Event::SearchInfinite => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::SearchInfinite);
            }
            Event::MaxTimePerMoveInMs(time) => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::MaxTimePerMoveInMs(time));
            }
            ref event @ Event::Moves(ref moves) => match moves_validation(moves) {
                Ok(valid_moves) => {
                    self.game_manager_actor
                        .do_send(game_manager::UciCommand::ValidMoves { moves: valid_moves });
                }
                Err(err) => {
                    actor_self.do_send(uci::UciResult::Err(HandleEventError::new(
                        event.clone(),
                        err,
                    )));
                }
            },
            Event::Wtime(wtime) => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::Wtime(wtime));
            }
            Event::Btime(btime) => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::Btime(btime));
            }
            Event::WtimeInc(wtime_inc) => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::WtimeInc(wtime_inc));
            }
            Event::BtimeInc(btime_inc) => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::BtimeInc(btime_inc));
            }
            ref event @ Event::SearchMoves(ref search_moves) => {
                match moves_validation(search_moves) {
                    Ok(valid_moves) => {
                        self.game_manager_actor
                            .do_send(game_manager::UciCommand::SearchMoves(valid_moves));
                    }
                    Err(err) => {
                        actor_self.do_send(uci::UciResult::Err(HandleEventError::new(
                            event.clone(),
                            err,
                        )));
                    }
                }
            }
            Event::StartEngine => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::EngineStartThinking);
                self.state_polling = uci::StatePollingUciEntity::Polling;
                ctx.address().do_send(uci::PollBestMove);
            }
            Event::StopEngine => {
                self.game_manager_actor
                    .do_send(game_manager::UciCommand::EngineStopThinking);
            }
            Event::Quit => {
                actor_self.do_send(uci::UciResult::Quit);
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
