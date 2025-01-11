use actix::{AsyncContext, Handler, Message};

use crate::monitoring::debug;

use super::UciEntity;

#[derive(Message)]
#[rtype(result = "Result<(), Vec<String>> ")]
pub struct ParseUserInput(pub String);
impl Handler<ParseUserInput> for UciEntity {
    type Result = Result<(), Vec<String>>;

    fn handle(&mut self, msg: ParseUserInput, ctx: &mut Self::Context) -> Self::Result {
        let errors = self.parse_input(&msg.0, ctx.address());
        if errors.is_empty() {
            Ok(())
        } else {
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!("Errors: {}", errors.join("\n"))));
            }
            Err(errors)
        }
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), Vec<String>> ")]
pub struct ReadUserInput;

impl Handler<ReadUserInput> for UciEntity {
    type Result = Result<(), Vec<String>>;

    fn handle(&mut self, _msg: ReadUserInput, ctx: &mut Self::Context) -> Self::Result {
        let mut errors: Vec<String> = vec![];
        if let Some(input) = self.uci_reader.uci_read() {
            let errs = self.parse_input(&input, ctx.address());
            errors.extend(errs);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            if let Some(debug_actor) = &self.debug_actor_opt {
                debug_actor.do_send(debug::AddMessage(format!("Errors: {}", errors.join("\n"))));
            }
            Err(errors)
        }
    }
}
