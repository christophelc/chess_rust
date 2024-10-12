use actix::{ActorContext, AsyncContext, Context, Handler, Message};

use super::Clock;

// Define a message to set the remaining time
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetRemainingTime {
    new_time: u64,
}
impl SetRemainingTime {
    pub fn new(new_time: u64) -> Self {
        SetRemainingTime { new_time }
    }
}
impl Handler<SetRemainingTime> for Clock {
    type Result = ();

    fn handle(&mut self, msg: SetRemainingTime, _ctx: &mut Context<Self>) {
        self.remaining_time = msg.new_time;
        println!("Clock time set to: {}", self.remaining_time);
    }
}

// Define a message to set the remaining time
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct IncRemainingTime(pub u64);
impl Handler<IncRemainingTime> for Clock {
    type Result = ();

    fn handle(&mut self, msg: IncRemainingTime, _ctx: &mut Context<Self>) {
        self.remaining_time += msg.0 * self.inc_time;
        println!(
            "Clock id '{}' time set to: {} after increment",
            self.id, self.remaining_time
        );
    }
}

// Define a message to set the remaining time
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct SetIncTime(u64);

impl SetIncTime {
    pub fn new(new_inc_time: u64) -> Self {
        SetIncTime(new_inc_time)
    }
}
impl Handler<SetIncTime> for Clock {
    type Result = ();

    fn handle(&mut self, msg: SetIncTime, _ctx: &mut Context<Self>) {
        self.inc_time = msg.0;
        println!("Clock inc time set to: {}", self.inc_time);
    }
}

// Define the message to pause the clock
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct PauseClock;

impl Handler<PauseClock> for Clock {
    type Result = ();

    fn handle(&mut self, _msg: PauseClock, ctx: &mut Context<Self>) {
        if let Some(handle) = self.ticking_handle.take() {
            ctx.cancel_future(handle); // Cancel the ticking interval to pause the clock
            println!("Clock paused");
        }
    }
}

// Define the message to resume the clock
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct ResumeClock;

impl Handler<ResumeClock> for Clock {
    type Result = ();

    fn handle(&mut self, _msg: ResumeClock, ctx: &mut Context<Self>) {
        if self.ticking_handle.is_none() {
            self.start_ticking(ctx); // Resume ticking if it was paused
            println!("Clock resumed");
        }
    }
}

#[derive(Debug, Message)]
#[rtype(result = "u64")]
pub struct GetRemainingTime;

impl Handler<GetRemainingTime> for Clock {
    type Result = u64;

    fn handle(&mut self, _msg: GetRemainingTime, _ctx: &mut Context<Self>) -> u64 {
        self.remaining_time
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct TerminateClock;

impl Handler<TerminateClock> for Clock {
    type Result = ();

    fn handle(&mut self, _msg: TerminateClock, ctx: &mut Context<Self>) {
        ctx.stop(); // stop the actor when a move is made
    }
}
