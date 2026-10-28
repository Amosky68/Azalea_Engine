use crate::gamestate::GameState;
use crate::moves::{Move, MoveGenerator};
use crate::eval::Evaluator;



const FUEL_MARGIN : u64 = 200_000_000;


pub struct Search {
    fuel_left: Box<dyn Fn() -> u64>,
    nodes: std::cell::Cell<u32>,
    stopped: std::cell::Cell<bool>,
}

impl Search {
    pub fn new(fuel_left: impl Fn() -> u64 + 'static) -> Search {
        Search { fuel_left: Box::new(fuel_left), nodes: std::cell::Cell::new(0), stopped: std::cell::Cell::new(false) }
    }

    #[inline]
    fn fuel_left(&self) -> u64 {
        (self.fuel_left)()
    }



    /// Choisit un coup. `seed` = la graine de l'arène : le SEUL hasard autorisé.
    pub fn get_best_move(&self, state: &GameState) -> Move {

        let root_maximizing = state.current_player == 0;
        self.nodes.set(0);
        self.stopped.set(false);

        // Iterative deepening
        let mut best_move = Move::pass_mv();
        for depth in 1..=10 {
            if self.stopped.get() { break; }

            let (_, mv) = self.alpha_beta(state, depth, i32::MIN, i32::MAX, root_maximizing);

            if self.stopped.get() { break; } // itération incomplète
            if mv != Move::pass_mv() {
                best_move = mv;
            }
        }
        best_move

    }


    pub fn alpha_beta(&self, state: &GameState, depth: i32, alpha: i32, beta: i32, maximizing_player: bool) -> (i32, Move) {
        if depth == 0 || state.is_game_over() || self.stopped.get() {
            return (Evaluator::evaluate(state), Move::pass_mv());
        }

        let n = self.nodes.get() + 1;
        self.nodes.set(n);
        if n % 64 == 0 && self.fuel_left() < FUEL_MARGIN {
            self.stopped.set(true);
            return (Evaluator::evaluate(state), Move::pass_mv());
        }

        let mut alpha = alpha;
        let mut beta = beta;
        let mut best_move = Move::pass_mv();

        let mut moves = MoveGenerator::generate_all_moves(state);
        Search::order_moves(&mut moves, state.current_player);


        if maximizing_player {
            let mut value = i32::MIN;
            
            for mv in moves {
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
            for mv in moves {
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


    pub fn order_moves(moves: &mut Vec<Move>, player: i16) {
        moves.sort_by_key(|mv| std::cmp::Reverse(Evaluator::move_ordering_evaluation(mv, player))); // ← Reverse = meilleurs EN PREMIER
    }



}