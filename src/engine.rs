use crate::uci;

pub struct EngineRandom {
    configuration: uci::Configuration,
}
impl EngineRandom {
    pub fn new() -> Self {
        EngineRandom {
            configuration: uci::Configuration::default(),
        }
    }
}
pub trait Engine {
    fn update(&mut self, new_configuration: &uci::Configuration) {
        println!("The configuration has changed");
    }
    fn configuration(&self) -> &uci::Configuration;
}

impl Engine for EngineRandom {
    fn configuration(&self) -> &uci::Configuration {
        &self.configuration
    }
    fn update(&mut self, new_configuration: &uci::Configuration) {
        println!("The configuration has changed");
        self.configuration = new_configuration.clone();
    }
}
