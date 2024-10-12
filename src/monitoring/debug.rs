use std::fmt;

use actix::{Actor, Addr, Context, Handler, Message};

pub struct DebugMessage(String);
pub struct DebugData {
    time: chrono::DateTime<chrono::Utc>,
    message: DebugMessage,
}
impl DebugData {
    pub fn new(message: DebugMessage) -> Self {
        Self {
            time: chrono::Utc::now(),
            message,
        }
    }
}
impl fmt::Display for DebugData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = format!("time:{}\tmessage:{}", self.time, self.message.0);
        write!(f, "{}", s)
    }
}

#[derive(Default)]
pub struct DebugEntity {
    stack: Vec<DebugData>,
    is_show: bool,
}
impl DebugEntity {
    #[allow(dead_code)]
    pub fn new(is_show: bool) -> Self {
        Self {
            stack: vec![],
            is_show,
        }
    }
    pub fn push(&mut self, message: DebugMessage) {
        let debug_data = DebugData::new(message);
        if self.is_show {
            println!("debug => {}", debug_data);
        }
        self.stack.push(debug_data);
    }
}
pub type DebugActor = Addr<DebugEntity>;

impl Actor for DebugEntity {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct AddMessage(pub String);

impl Handler<AddMessage> for DebugEntity {
    type Result = ();

    fn handle(&mut self, msg: AddMessage, _ctx: &mut Self::Context) -> Self::Result {
        let debug_message = DebugMessage(msg.0);
        self.push(debug_message)
    }
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct ShowAllMessages;

impl Handler<ShowAllMessages> for DebugEntity {
    type Result = Vec<String>;

    fn handle(&mut self, _msg: ShowAllMessages, _ctx: &mut Self::Context) -> Self::Result {
        let result = self.stack.iter().map(|el| el.to_string()).collect();
        result
    }
}
