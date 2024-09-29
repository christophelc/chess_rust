use crate::board::square;

use super::engine;

pub enum Player<T: engine::EngineActor> {
    Human {
        engine_opt: Option<Box<T>>, // even if human, we can ask to get a hint to the engine
    },
    Computer {
        engine: Box<T>,
    },
}
impl<T: engine::EngineActor> Default for Player<T> {
    fn default() -> Self {
        Player::Human { engine_opt: None }
    }
}
impl<T: engine::EngineActor> Player<T> {
    pub fn get_engine(&self) -> Option<&Box<T>> {
        match self {
            Player::Human { engine_opt: None } => None,
            Player::Human {
                engine_opt: Some(engine),
            } => Some(engine),
            Player::Computer { engine } => Some(engine),
        }
    }
}

#[derive(Default)]
pub struct Players<T: engine::EngineActor> {
    white: Player<T>,
    black: Player<T>,
}
impl<T: engine::EngineActor> Players<T> {
    pub fn new(white: Player<T>, black: Player<T>) -> Self {
        Players { white, black }
    }
    pub fn get_player_into(&mut self, color: square::Color) -> &mut Player<T> {
        if color == square::Color::White {
            &mut self.white
        } else {
            &mut self.black
        }
    }
    pub fn get_engine(&mut self, color: square::Color) -> Result<&Box<T>, String> {
        let player = self.get_player_into(color);
        match player.get_engine() {
            None => Err(format!("No engine for player {:?}", color)),
            Some(engine) => Ok(engine),
        }
    }
}
