pub mod parser;
use actix::{Addr, Message};
use std::error::Error;

use crate::entity::game::actor::game_manager;

#[derive(Debug)]
pub struct CommandError {
    command: String,
}
impl CommandError {
    pub fn new(command: String) -> Self {
        CommandError { command }
    }
}
impl Error for CommandError {}
impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Command error for command: '{}'", self.command)
    }
}

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub enum Command {
    DebugMode(bool),
    Go(GoStruct),
    Ignore,  // do nothing
    IsReady, // "isready" command, no additional data needed
    NewGame,
    Position(PositionStruct),
    Quit,                                 // "quit" command to exit the engine
    Stop,                                 // "stop" command to stop search
    Uci(Addr<game_manager::GameManager>), // "uci" command, no additional data needed
    Wait100ms,                            // for test purpose
}

#[derive(Debug, Default)]
pub struct PositionStruct {
    // "position" command, with optional FEN and moves
    startpos: bool,      // `true` if the starting position is requested
    fen: Option<String>, // The FEN string, if specified (None if using startpos)
    moves: Vec<String>,  // A list of moves played after the position
}
impl PositionStruct {
    pub fn startpos(&self) -> bool {
        self.startpos
    }
    pub fn fen(&self) -> Option<String> {
        self.fen.clone()
    }
    pub fn moves(&self) -> &Vec<String> {
        &self.moves
    }
}

#[derive(Debug, Default)]
pub struct GoStruct {
    // "go" command, with search parameters
    depth: Option<u32>,        // Optional depth to search
    movetime: Option<u32>,     // Optional maximum time for the move (in ms)
    infinite: bool,            // If true, search indefinitely until told to stop
    wtime: Option<u64>,        // White time left,
    btime: Option<u64>,        // Black time left
    wtime_inc: Option<u64>,    // White time increment per move
    btime_inc: Option<u64>,    // Black time increment per move
    search_moves: Vec<String>, // Restrict search to this moves only
}

impl GoStruct {
    pub fn depth(&self) -> Option<u32> {
        self.depth
    }
    pub fn movetime(&self) -> Option<u32> {
        self.movetime
    }
    pub fn infinite(&self) -> bool {
        self.infinite
    }
    pub fn wtime(&self) -> Option<u64> {
        self.wtime
    }
    pub fn btime(&self) -> Option<u64> {
        self.btime
    }
    pub fn wtime_inc(&self) -> Option<u64> {
        self.wtime_inc
    }
    pub fn btime_inc(&self) -> Option<u64> {
        self.btime_inc
    }
    pub fn search_moves(&self) -> &Vec<String> {
        &self.search_moves
    }
}
