use actix::Addr;

use crate::board::square;

use super::engine::{EngineDispatcher, EngineDispatcherActor};

pub enum Player {
    Human {
        engine_opt: Option<Addr<EngineDispatcher>>, // even if human, we can ask to get a hint to the engine
    },
    Computer {
        engine: Addr<EngineDispatcher>,
    },
}
impl Default for Player {
    fn default() -> Self {
        Player::Human { engine_opt: None }
    }
}
impl Player {
    pub fn get_engine(&self) -> Option<&EngineDispatcherActor> {
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
pub struct Players {
    white: Player,
    black: Player,
}
impl Players {
    pub fn new(white: Player, black: Player) -> Self {
        Players { white, black }
    }
    pub fn get_player_into(&mut self, color: square::Color) -> &mut Player {
        if color == square::Color::White {
            &mut self.white
        } else {
            &mut self.black
        }
    }
    fn get_player(&self, color: square::Color) -> &Player {
        if color == square::Color::White {
            &self.white
        } else {
            &self.black
        }
    }
    pub fn get_engine(&self, color: square::Color) -> Result<&EngineDispatcherActor, String> {
        let player = self.get_player(color);
        match player.get_engine() {
            None => Err(format!("No engine for player {:?}", color)),
            Some(engine) => Ok(engine),
        }
    }
}
