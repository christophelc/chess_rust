use actix::prelude::*;

use crate::entity::engine::actor::engine_dispatcher as dispatcher;
use crate::entity::game::component::bitboard::piece_move::GenMoves;
use crate::entity::game::component::{bitboard, game_state};

#[derive(Debug, Clone)]
pub struct EngineId {
    name: String,
    author: String,
}
impl EngineId {
    pub fn new(name: String, author: String) -> Self {
        Self { name, author }
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn author(&self) -> String {
        self.author.clone()
    }
}
pub trait Engine {
    fn id(&self) -> EngineId;

    fn find_best_move(
        &self,
        self_actor: Addr<dispatcher::EngineDispatcher>,
        game: game_state::GameState,
    );
}

pub fn gen_moves(bit_position: &bitboard::BitPosition) -> Vec<bitboard::BitBoardMove> {
    let bit_boards_white_and_black = bit_position.bit_boards_white_and_black();
    let bit_position_status = bit_position.bit_position_status();
    let color = &bit_position_status.player_turn();
    let check_status = bit_boards_white_and_black.check_status(color);
    let capture_en_passant = bit_position_status.pawn_en_passant();
    bit_boards_white_and_black.gen_moves_for_all(
        color,
        check_status,
        capture_en_passant.as_ref(),
        bit_position_status,
    )
}
