use actix::prelude::*;
use std::time::Duration;

use super::engine;

pub type ClockActor<T> = Addr<Clock<T>>;
pub struct Clock<T: engine::EngineActor> {
    id: String, // useful for debug
    remaining_time: u64,
    inc_time: u64,
    game_actor: super::GameActor<T>,
    ticking_handle: Option<SpawnHandle>, // Handle to the ticking interval
}

impl<T: engine::EngineActor> Clock<T> {
    #[cfg(test)]
    pub fn new(
        id: &str,
        starting_time: u64,
        inc_time: u64,
        game_actor: super::GameActor<T>,
    ) -> Self {
        Clock {
            id: id.to_string(),
            remaining_time: starting_time,
            inc_time,
            game_actor,
            ticking_handle: None,
        }
    }
    // Start ticking every second, reducing remaining time
    fn start_ticking(&mut self, ctx: &mut Context<Self>) {
        // Save the handle for the ticking interval so it can be paused later
        let ticking_handle = ctx.run_interval(Duration::from_secs(1), |clock, ctx| {
            if clock.remaining_time > 0 {
                clock.remaining_time -= 1;
                println!("Remaining time: {}", clock.remaining_time);
            } else {
                println!("No remaining time.");
                clock.game_actor.do_send(TimeOut);
                ctx.stop(); // Stop the actor when time is up
            }
        });
        self.ticking_handle = Some(ticking_handle);
    }
}

impl<T: engine::EngineActor> Actor for Clock<T> {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        // we do nothing: we do not start yet the timer
    }
}

// Define a message to set the remaining time
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetRemainingTime {
    new_time: u64,
}
impl SetRemainingTime {
    pub fn new(new_time: u64) -> Self {
        SetRemainingTime { new_time }
    }
}
impl<T: engine::EngineActor> Handler<SetRemainingTime> for Clock<T> {
    type Result = ();

    fn handle(&mut self, msg: SetRemainingTime, _ctx: &mut Context<Self>) {
        self.remaining_time = msg.new_time;
        println!("Clock time set to: {}", self.remaining_time);
    }
}

// Define a message to set the remaining time
#[derive(Message)]
#[rtype(result = "()")]
pub struct IncRemainingTime {}
impl IncRemainingTime {
    pub fn new() -> Self {
        IncRemainingTime {}
    }
}
impl<T: engine::EngineActor> Handler<IncRemainingTime> for Clock<T> {
    type Result = ();

    fn handle(&mut self, _msg: IncRemainingTime, _ctx: &mut Context<Self>) {
        self.remaining_time += self.inc_time;
        println!(
            "Clock id '{}' time set to: {} after increment",
            self.id, self.remaining_time
        );
    }
}

// Define a message to set the remaining time
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetIncTime(u64);

impl SetIncTime {
    pub fn new(new_inc_time: u64) -> Self {
        SetIncTime(new_inc_time)
    }
}
impl<T: engine::EngineActor> Handler<SetIncTime> for Clock<T> {
    type Result = ();

    fn handle(&mut self, msg: SetIncTime, _ctx: &mut Context<Self>) {
        self.inc_time = msg.0;
        println!("Clock inc time set to: {}", self.inc_time);
    }
}

// Define the message to pause the clock
#[derive(Message)]
#[rtype(result = "()")]
pub struct PauseClock;

impl<T: engine::EngineActor> Handler<PauseClock> for Clock<T> {
    type Result = ();

    fn handle(&mut self, _msg: PauseClock, ctx: &mut Context<Self>) {
        if let Some(handle) = self.ticking_handle.take() {
            ctx.cancel_future(handle); // Cancel the ticking interval to pause the clock
            println!("Clock paused");
        }
    }
}

// Define the message to resume the clock
#[derive(Message)]
#[rtype(result = "()")]
pub struct ResumeClock;

impl<T: engine::EngineActor> Handler<ResumeClock> for Clock<T> {
    type Result = ();

    fn handle(&mut self, _msg: ResumeClock, ctx: &mut Context<Self>) {
        if self.ticking_handle.is_none() {
            self.start_ticking(ctx); // Resume ticking if it was paused
            println!("Clock resumed");
        }
    }
}

#[derive(Message)]
#[rtype(result = "u64")]
pub struct GetRemainingTime;

impl<T: engine::EngineActor> Handler<GetRemainingTime> for Clock<T> {
    type Result = u64;

    fn handle(&mut self, _msg: GetRemainingTime, _ctx: &mut Context<Self>) -> u64 {
        self.remaining_time
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TerminateClock;

impl<T: engine::EngineActor> Handler<TerminateClock> for Clock<T> {
    type Result = ();

    fn handle(&mut self, _msg: TerminateClock, ctx: &mut Context<Self>) {
        ctx.stop(); // stop the actor when a move is made
    }
}

// Message sent to game actor when clock runs out
#[derive(Message)]
#[rtype(result = "()")]
pub struct TimeOut;
