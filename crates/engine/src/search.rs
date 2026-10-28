use azalea_core::gamestate::GameState;
use azalea_core::moves::{Move, MoveGenerator};
use crate::eval::Evaluator;


pub struct Search;

impl Search {
    /// Choisit un coup. `seed` = la graine de l'arène : le SEUL hasard autorisé.
    pub fn get_best_move(&self, state: &GameState) -> Move {
        
        let depth = 2; // Profondeur de recherche
        let (_, best_move) = self.alpha_beta(state, depth, i32::MIN, i32::MAX, true);
        best_move
    }


    pub fn alpha_beta(&self, state: &GameState, depth: i32, alpha: i32, beta: i32, maximizing_player: bool) -> (i32, Move) {
        if depth == 0 || state.is_game_over() {
            return (Evaluator::evaluate(state), Move::pass_mv());
        }

        let mut alpha = alpha;
        let mut beta = beta;
        let mut best_move = Move::pass_mv();

        if maximizing_player {
            let mut value = i32::MIN;
            for mv in MoveGenerator::generate_all_moves(state) {
                let new_state = state.apply_move(&mv);
                let (eval, _) = self.alpha_beta(&new_state, depth - 1, alpha, beta, false);
                
                if eval > value {
                    value = eval;
                    best_move = mv;
                }
                
                alpha = alpha.max(value);
                if beta <= alpha {
                    break; // β cutoff
                }
            }
            return (value, best_move);
        
        
        } else {
            let mut value = i32::MAX;
            for mv in MoveGenerator::generate_all_moves(state) {
                let new_state = state.apply_move(&mv);
                let (eval, _) = self.alpha_beta(&new_state, depth - 1, alpha, beta, true);
                
                if eval < value {
                    value = eval;
                    best_move = mv;
                }

                beta = beta.min(value);
                if beta <= alpha {
                    break; // α cutoff
                }
            }
            return (value, best_move);
        }
    }


    



}