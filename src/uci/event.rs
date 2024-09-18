use std::{error::Error, io::Stdout};

use super::{notation, UciResult};
use crate::board::fen;
use crate::board::fen::EncodeUserInput;
use crate::game;
use std::io::Write;

#[derive(Debug, Clone)]
pub enum Event {
    Btime(u64),
    Depth(u32),
    Fen(String),
    Moves(Vec<String>),
    Quit,
    SearchMoves(Vec<String>),
    SearchInfinite,
    StartPos,
    Stop,
    TimePerMoveInMs(u32),
    Write(String),
    Wtime(u64),
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
    pub fn event(&self) -> &Event {
        &self.event
    }
    pub fn error(&self) -> &str {
        &self.error
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

fn game_cast_result(
    event: &Event,
    result: Result<Result<(), String>, actix::MailboxError>,
) -> Result<(), HandleEventError> {
    if let Some(mailbox_err) = result.clone().err() {
        Err(HandleEventError::new(
            event.clone(),
            mailbox_err.to_string(),
        ))
    } else if let Some(event_err) = result.unwrap().err() {
        Err(HandleEventError::new(event.clone(), event_err))
    } else {
        Ok(())
    }
}

impl Event {
    pub async fn handle_event(
        &self,
        game: &game::GameActor,
        stdout: &mut Stdout,
    ) -> Result<UciResult, HandleEventError> {
        match self {
            Event::Write(s) => {
                writeln!(stdout, "{}", s).unwrap();
                stdout.flush().unwrap();
                Ok(UciResult::Continue)
            }
            Event::StartPos => {
                let r = (*game).send(game::UciCommand::InitPosition).await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            Event::Fen(fen) => {
                let position = fen::Fen::decode(fen).expect("Failed to decode FEN");
                let r = (*game)
                    .send(game::UciCommand::UpdatePosition(fen.to_string(), position))
                    .await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            // Go command
            Event::Depth(depth) => {
                let r = (*game).send(game::UciCommand::DepthFinite(*depth)).await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            Event::SearchInfinite => {
                let r = (*game).send(game::UciCommand::SearchInfinite).await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            Event::TimePerMoveInMs(time) => {
                let r = (*game).send(game::UciCommand::TimePerMoveInMs(*time)).await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            event @ Event::Moves(moves) => match moves_validation(moves) {
                Ok(valid_moves) => {
                    let r = (*game)
                        .send(game::UciCommand::ValidMoves(valid_moves))
                        .await;
                    game_cast_result(self, r).map(|_| UciResult::Continue)
                }
                Err(err) => Err(HandleEventError::new(event.clone(), err)),
            },
            Event::Wtime(wtime) => {
                let r = (*game).send(game::UciCommand::Wtime(*wtime)).await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            Event::Btime(btime) => {
                let r = (*game).send(game::UciCommand::Btime(*btime)).await;
                game_cast_result(self, r).map(|_| UciResult::Continue)
            }
            event @ Event::SearchMoves(search_moves) => match moves_validation(search_moves) {
                Ok(valid_moves) => {
                    let r = (*game)
                        .send(game::UciCommand::SearchMoves(valid_moves))
                        .await;
                    game_cast_result(self, r).map(|_| UciResult::Continue)
                }
                Err(err) => Err(HandleEventError::new(event.clone(), err.to_string())),
            },
            Event::Stop => {
                let r = (*game).send(game::UciCommand::Stop).await;
                game_cast_result(self, r).map(|_| UciResult::BestMove)
            }
            Event::Quit => Ok(UciResult::Quit),
        }
    }
}

pub fn moves_validation(
    moves: &Vec<String>,
) -> Result<Vec<notation::LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<notation::LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match notation::LongAlgebricNotationMove::build_from_str(m) {
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
