pub mod handler_clock;

use actix::prelude::*;
use std::time::Duration;

use crate::entity::game::actor::game_manager;

pub type ClockActor = Addr<Clock>;
pub struct Clock {
    id: String, // useful for debug
    remaining_time: u64,
    inc_time: u64,
    game_actor: game_manager::GameManagerActor,
    ticking_handle: Option<SpawnHandle>, // Handle to the ticking interval
}

impl Clock {
    pub fn new(
        id: &str,
        starting_time: u64,
        inc_time: u64,
        game_actor: game_manager::GameManagerActor,
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
                clock
                    .game_actor
                    .do_send(game_manager::handler_clock::TimeOut);
                ctx.stop(); // Stop the actor when time is up
            }
        });
        self.ticking_handle = Some(ticking_handle);
    }
}

impl Actor for Clock {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        // we do nothing: we do not start yet the timer
    }
}
