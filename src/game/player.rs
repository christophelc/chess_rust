use crate::board::square;

use super::engine;

pub enum Player<T: engine::EngineActor> {
    Human(Option<Box<T>>), // even if human, we can ask to get a hint to the engine
    Computer(Box<T>),
}
impl<T: engine::EngineActor> Default for Player<T> {
    fn default() -> Self {
        Player::Human(None)
    }
}
impl<T: engine::EngineActor> Player<T> {
    pub fn get_engine(&self) -> Option<&Box<T>> {
        match self {
            Player::Human(None) => None,
            Player::Human(Some(engine)) => Some(engine),
            Player::Computer(engine) => Some(engine),
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
    pub fn set_white(&mut self, white: Player<T>) {
        self.white = white;
    }
    pub fn set_black(&mut self, black: Player<T>) {
        self.black = black;
    }
    fn get_player(&self, color: square::Color) -> &Player<T> {
        if color == square::Color::White {
            &self.white
        } else {
            &self.black
        }
    }
    pub fn get_engine(&mut self, color: square::Color) -> Result<&Box<T>, String> {
        let player = self.get_player(color);
        match player.get_engine() {
            None => Err(format!("No engine for player {:?}", color)),
            Some(engine) => Ok(engine),
        }
    }
}
