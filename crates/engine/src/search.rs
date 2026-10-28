use azalea_core::gamestate::GameState;
use azalea_core::moves::{Move, MoveGenerator};

pub struct Search;

impl Search {
    /// Choisit un coup. `seed` = la graine de l'arène : le SEUL hasard autorisé.
    pub fn get_best_move(&self, state: &GameState, seed: u64) -> Move {
        let moves = MoveGenerator::generate_all_moves(state);

        // xorshift64 seedé par la graine du tour (placeholder aléatoire)
        let mut r = seed | 1;
        r ^= r << 13;
        r ^= r >> 7;
        r ^= r << 17;
        moves[(r % moves.len() as u64) as usize]
    }
}