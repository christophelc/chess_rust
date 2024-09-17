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

impl Event {
    pub async fn handle_event(
        &self,
        game: &game::GameActor,
        stdout: &mut Stdout,
    ) -> Result<UciResult, HandleEventError> {
        let mut result = Ok(UciResult::Continue);
        match self {
            Event::Write(s) => {
                writeln!(stdout, "{}", s).unwrap();
                stdout.flush().unwrap();
            }
            Event::StartPos => {
                let _ = (*game).send(game::UciCommand::InitPosition).await;
            }
            Event::Fen(fen) => {
                let position = fen::FEN::decode(fen).expect("Failed to decode FEN");
                let _ = (*game)
                    .send(game::UciCommand::UpdatePosition(position))
                    .await;
            }
            // Go command
            Event::Depth(depth) => {
                let _ = (*game).send(game::UciCommand::DepthFinite(*depth)).await;
            }
            Event::SearchInfinite => {
                let _ = (*game).send(game::UciCommand::SearchInfinite).await;
            }
            Event::TimePerMoveInMs(time) => {
                let _ = (*game).send(game::UciCommand::TimePerMoveInMs(*time)).await;
            }
            event @ Event::Moves(moves) => match moves_validation(moves) {
                Ok(valid_moves) => {
                    let _ = (*game)
                        .send(game::UciCommand::ValidMoves(valid_moves))
                        .await;
                }
                Err(err) => result = Err(HandleEventError::new(event.clone(), err)),
            },
            Event::Wtime(wtime) => {
                let _ = (*game).send(game::UciCommand::Wtime(*wtime)).await;
            }
            Event::Btime(btime) => {
                let _ = (*game).send(game::UciCommand::Btime(*btime)).await;
            }
            event @ Event::SearchMoves(search_moves) => match moves_validation(search_moves) {
                Ok(valid_moves) => {
                    let _ = (*game)
                        .send(game::UciCommand::SearchMoves(valid_moves))
                        .await;
                }
                Err(err) => result = Err(HandleEventError::new(event.clone(), err.to_string())),
            },
            Event::Stop => {
                let _ = (*game).send(game::UciCommand::Stop).await;
                result = Ok(UciResult::BestMove);
            }
            Event::Quit => result = Ok(UciResult::Quit),
        }
        result
    }
}

pub fn moves_validation(
    moves: &Vec<String>,
) -> Result<Vec<notation::LongAlgebricNotationMove>, String> {
    let mut valid_moves: Vec<notation::LongAlgebricNotationMove> = vec![];
    let mut errors: Vec<String> = vec![];
    for m in moves {
        match notation::LongAlgebricNotationMove::build_from_str(&m) {
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
