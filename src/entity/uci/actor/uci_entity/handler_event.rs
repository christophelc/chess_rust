use actix::{dev::ContextFutureSpawner, AsyncContext, Handler, Message, WrapFuture};

use crate::entity::uci::component::event;

use super::{UciEntity, UciRead};

#[derive(Message)]
#[rtype(result = "()")]
pub struct ProcessEvents(pub Vec<event::Event>);

impl<R: UciRead> Handler<ProcessEvents> for UciEntity<R> {
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
