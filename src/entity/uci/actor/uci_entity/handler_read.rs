use actix::{AsyncContext, Handler, Message};

use crate::{entity::uci::component::command::parser, monitoring::debug};

use super::UciEntity;

#[derive(Message)]
#[rtype(result = "Result<(), Vec<String>> ")]
pub struct ReadUserInput;

impl Handler<ReadUserInput> for UciEntity {
    type Result = Result<(), Vec<String>>;

    fn handle(&mut self, _msg: ReadUserInput, ctx: &mut Self::Context) -> Self::Result {
        let mut errors: Vec<String> = vec![];
        if let Some(input) = self.uci_reader.uci_read() {
            let parser = parser::InputParser::new(&input, self.game_manager_actor.clone());
            let command_or_error = parser.parse_input();
            match command_or_error {
                Ok(command) => {
                    if let Some(debug_actor) = &self.debug_actor_opt {
                        debug_actor.do_send(debug::AddMessage(format!(
                            "input '{}' send as command '{:?}' to uci_actor",
                            input, command
                        )));
                    }
                    ctx.address().do_send(command);
                }
                Err(err) => errors.push(err.to_string()),
            }
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
