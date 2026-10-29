use crate::gamestate::GameState;
use crate::moves::{Move, MoveGenerator};
use crate::eval::Evaluator;
use crate::zobrist::{Zobrist, Bounds};



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

        let mut zobrist: Zobrist = Zobrist::new();

        // Iterative deepening
        let mut best_move = Move::pass_mv();
        for depth in 1..=10 {
            if self.stopped.get() { break; }

            // zobrist.clear();
            let (_, mv) = self.alpha_beta(state, depth, i32::MIN, i32::MAX, root_maximizing, &mut zobrist);

            if self.stopped.get() { break; } // itération incomplète
            if mv != Move::pass_mv() {
                best_move = mv;
            }
        }
        best_move

    }


    pub fn alpha_beta(&self, state: &GameState, depth: i32, mut alpha: i32, mut beta: i32, maximizing_player: bool, zobrist: &mut Zobrist) -> (i32, Move) {
        let original_alpha = alpha;
        let original_beta = beta;
        

        let mut _tt_move = Move::pass_mv();
        if let Some(entry) = zobrist.probe(state) {
            _tt_move = entry.best_move; // On mémorise le coup pour le Move Ordering
            if entry.depth >= depth {
                match entry.bound {
                    Bounds::Exact => return (entry.score, entry.best_move),
                    Bounds::Lower => alpha = alpha.max(entry.score),
                    Bounds::Upper => beta = beta.min(entry.score),
                }
                if alpha >= beta {
                    return (entry.score, entry.best_move);
                }
            }
        }

        
        if depth == 0 || state.is_game_over() || self.stopped.get() {
            return (Evaluator::evaluate(state), Move::pass_mv());
        }

        // Fuel check
        let n = self.nodes.get() + 1;
        self.nodes.set(n);
        if n % 64 == 0 && self.fuel_left() < FUEL_MARGIN {
            self.stopped.set(true);
            return (Evaluator::evaluate(state), Move::pass_mv());
        }

        let mut best_move = Move::pass_mv();
        let mut best_score = if maximizing_player { i32::MIN } else { i32::MAX };

        let mut moves = MoveGenerator::generate_all_moves(state); // Todo (prendre en compte le coup du transposition table pour le Move Ordering)
        Search::order_moves(&mut moves, _tt_move, state.current_player);


        if maximizing_player {
            for mv in moves {
                let new_state = state.apply_move(&mv);
                let (eval, _) = self.alpha_beta(&new_state, depth - 1, alpha, beta, false, zobrist);
                
                if eval > best_score {
                    best_score = eval;
                    best_move = mv;
                }
                alpha = alpha.max(best_score);
                if beta <= alpha { break; } // Beta cutoff
                }
            }
        else {
            for mv in moves {
                let new_state = state.apply_move(&mv);
                let (eval, _) = self.alpha_beta(&new_state, depth - 1, alpha, beta, true, zobrist);

                if eval < best_score {
                    best_score = eval;
                    best_move = mv;
                }

                beta = beta.min(best_score);
                if beta <= alpha { break; }// Alpha cutoff 
            }
        }

        // Store in transposition table
        if !self.stopped.get() {
            let bound = if best_score <= original_alpha {
                Bounds::Upper
            } else if best_score >= original_beta {
                Bounds::Lower
            } else {
                Bounds::Exact
            };
            zobrist.add(state, depth, best_score, best_move, bound);
        }

        (best_score, best_move)
    }



    pub fn order_moves(moves: &mut Vec<Move>, tt_move : Move, player: i16) {
        moves.sort_by_key(|mv| std::cmp::Reverse(Evaluator::move_ordering_evaluation(mv, tt_move,player))); 
    }



}