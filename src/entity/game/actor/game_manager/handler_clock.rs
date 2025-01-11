use actix::{Context, Handler, Message, ResponseFuture};

use crate::{
    entity::{
        clock::actor::chessclock,
        game::component::{
            game_state,
            square::{self, Switch},
        },
    },
    monitoring::debug,
};

use super::GameManager;

use crate::span_debug;

fn span_debug() -> tracing::Span {
    span_debug!("game_manager::handler_clock")
}

// Message to set the clocks in the Game actor
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetClocks {
    white_clock_actor_opt: Option<chessclock::ClockActor>,
    black_clock_actor_opt: Option<chessclock::ClockActor>,
}
impl SetClocks {
    pub fn new(
        white_clock_actor_opt: Option<chessclock::ClockActor>,
        black_clock_actor_opt: Option<chessclock::ClockActor>,
    ) -> Self {
        SetClocks {
            white_clock_actor_opt,
            black_clock_actor_opt,
        }
    }
}
impl Handler<SetClocks> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: SetClocks, _ctx: &mut Context<Self>) {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        // Set the new clock actors from the message
        if msg.white_clock_actor_opt.is_some() {
            // If white clock exists, terminate it before setting a new one
            if let Some(clock_actor) = &self.white_clock_actor_opt {
                clock_actor.do_send(chessclock::handler_clock::TerminateClock);
            }
            self.white_clock_actor_opt = msg.white_clock_actor_opt;
        }
        if msg.black_clock_actor_opt.is_some() {
            // If black clock exists, terminate it before setting a new one
            if let Some(clock_actor) = &self.black_clock_actor_opt {
                clock_actor.do_send(chessclock::handler_clock::TerminateClock);
            }
            self.black_clock_actor_opt = msg.black_clock_actor_opt;
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "Option<u64>")]
pub struct GetClockRemainingTime(square::Color);

impl GetClockRemainingTime {
    pub fn new(color: square::Color) -> Self {
        GetClockRemainingTime(color)
    }
}

impl Handler<GetClockRemainingTime> for GameManager {
    type Result = ResponseFuture<Option<u64>>;

    fn handle(&mut self, msg: GetClockRemainingTime, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        let white_clock_actor_opt = self.white_clock_actor_opt.clone();
        let black_clock_actor_opt = self.black_clock_actor_opt.clone();
        Box::pin(async move {
            match (msg.0, white_clock_actor_opt, black_clock_actor_opt) {
                (square::Color::White, Some(white_clock_actor), _) => {
                    let result = white_clock_actor
                        .send(chessclock::handler_clock::GetRemainingTime)
                        .await
                        .ok()?;
                    Some(result)
                }
                (square::Color::Black, _, Some(black_clock_actor)) => {
                    let result = black_clock_actor
                        .send(chessclock::handler_clock::GetRemainingTime)
                        .await
                        .ok()?;
                    Some(result)
                }
                _ => None,
            }
        })
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetClockRemainingTime {
    color: square::Color,
    remaining_time: u64,
}
impl SetClockRemainingTime {
    #[cfg(test)]
    pub fn new(color: &square::Color, remaining_time: u64) -> Self {
        Self {
            color: color.clone(),
            remaining_time,
        }
    }
}

impl Handler<SetClockRemainingTime> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: SetClockRemainingTime, _ctx: &mut Self::Context) -> Self::Result {
        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        match msg.color {
            square::Color::White => self.white_clock_actor_opt.as_mut().unwrap().do_send(
                chessclock::handler_clock::SetRemainingTime::new(msg.remaining_time),
            ),
            square::Color::Black => self.black_clock_actor_opt.as_mut().unwrap().do_send(
                chessclock::handler_clock::SetRemainingTime::new(msg.remaining_time),
            ),
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct StartOrSwitchClocks;

// Implementing a handler for starting the clocks
impl Handler<StartOrSwitchClocks> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: StartOrSwitchClocks, _ctx: &mut Context<Self>) {
        let span = span_debug();
        let _enter = span.enter();

        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        if let Some(game_state) = &self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            if self.white_clock_actor_opt.is_none() || self.black_clock_actor_opt.is_none() {
                panic!("Cannot start clocks. No clock has been defined.")
            }
            match color {
                square::Color::White => {
                    tracing::debug!("Pause black, resume white");
                    self.black_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::handler_clock::PauseClock);
                    self.white_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::handler_clock::ResumeClock);
                }
                square::Color::Black => {
                    tracing::debug!("Pause white, resume black");
                    self.black_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::handler_clock::ResumeClock);
                    self.white_clock_actor_opt
                        .as_mut()
                        .unwrap()
                        .do_send(chessclock::handler_clock::PauseClock);
                }
            }
        } else {
            panic!("Try to start clocks whereas no position has been detected.")
        }
    }
}

// Message sent to game actor when clock runs out
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct TimeOut;
impl Handler<TimeOut> for GameManager {
    type Result = ();

    fn handle(&mut self, msg: TimeOut, _ctx: &mut Context<Self>) {
        let span = span_debug();
        let _enter = span.enter();

        if let Some(debug_actor) = &self.debug_actor_opt {
            debug_actor.do_send(debug::AddMessage(format!(
                "game_manager_actor receive {:?}",
                msg
            )));
        }
        if let Some(ref mut game_state) = &mut self.game_state_opt {
            let bitboard_position = game_state.bit_position();
            let bit_position_status = bitboard_position.bit_position_status();
            let color = bit_position_status.player_turn();
            let bit_boards_white_and_black = bitboard_position.bit_boards_white_and_black();
            if game_state
                .check_insufficient_material_for_color(color.switch(), bit_boards_white_and_black)
            {
                game_state.set_end_game(game_state::EndGame::TimeOutDraw);
                tracing::debug!("set end game TimeOutDraw");
            } else {
                game_state.set_end_game(game_state::EndGame::TimeOutLost(color));
                tracing::debug!("set end game: TimeOutLost");
            }
        } else {
            panic!("A clock has been started but no position has been set.")
        }
    }
}
