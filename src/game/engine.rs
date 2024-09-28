use actix::prelude::*;

// TODO: engine actor
pub trait Engine {
    // start thinking
    fn go(&self);
    // stop thinking
    fn stop(&self);
}
pub trait EngineActor: Actor + Engine + Default + Clone {}

// Implementation
#[derive(Debug, Clone, Default)]
pub struct EngineDummy {}
impl Actor for EngineDummy {
    type Context = Context<Self>;
}
impl EngineDummy {
    pub fn new() -> Self {
        EngineDummy {}
    }
}
impl EngineActor for EngineDummy {}
impl Engine for EngineDummy {
    fn go(&self) {
        println!("EngineDummy started thinking.");
    }
    fn stop(&self) {
        println!("EngineDummy stopped thinking.");
    }
}

pub struct EngineGo;
impl Message for EngineGo {
    type Result = ();
}

pub struct EngineStop;
impl Message for EngineStop {
    type Result = ();
}

// Handle the Go and Stop messages
impl Handler<EngineGo> for EngineDummy {
    type Result = ();

    fn handle(&mut self, _msg: EngineGo, _ctx: &mut Self::Context) {
        self.go();
    }
}

impl Handler<EngineStop> for EngineDummy {
    type Result = ();

    fn handle(&mut self, _msg: EngineStop, _ctx: &mut Self::Context) {
        self.stop();
    }
}
