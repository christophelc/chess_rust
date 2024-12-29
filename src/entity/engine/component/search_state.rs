use crate::entity::game::component::bitboard;


pub const MAX_DEPTH: usize = 64;

#[derive(Debug)]
pub struct SearchState {
    // Stocke les killer moves pour chaque profondeur.
    killer_moves: [[Option<bitboard::BitBoardMove>; 2]; MAX_DEPTH],
}

impl SearchState {
    pub fn new() -> Self {
        SearchState {
            killer_moves: [[None; 2]; MAX_DEPTH],
        }
    }

    // Ajoute un killer move pour une profondeur donnée
    pub fn add_killer_move(&mut self, depth: usize, mv: bitboard::BitBoardMove) {
        // Vérifie si le coup est déjà stocké
        if self.killer_moves[depth][0] != Some(mv) {
            // Déplace le premier coup dans la deuxième position et ajoute le nouveau
            self.killer_moves[depth][1] = self.killer_moves[depth][0];
            self.killer_moves[depth][0] = Some(mv);
        }
    }

    // Vérifie si un coup est un killer move
    pub fn is_killer_move(&self, depth: usize, mv: bitboard::BitBoardMove) -> bool {
        self.killer_moves[depth].contains(&Some(mv))
    }
}
