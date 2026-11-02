use crate::gamestate::GameState;
use crate::moves::{Move, MoveGenerator};
use crate::eval::Evaluator;
use crate::zobrist::{Zobrist, Bounds};
use crate::BitboardsUtils::Utils;
use crate::board::Board;
use std::cell::{Cell, RefCell};



const FUEL_MARGIN : u64 = 100_000_000;
const QSEARCH_MAX_DEPTH: i32 = 4;

const LMR_MIN_DEPTH : i32  = 3;  // jamais de réduction au-dessus de l'horizon rapproché
const LMR_FIRST_MOVE: usize = 4; // jamais sur les 4 premiers coups (les mieux ordonnés)



#[derive(Debug, Clone, Copy, Default)]
pub struct SearchStats {
    pub nodes: u64,       // nœuds visités ce coup
    pub depth: i32,       // dernière profondeur COMPLÈTE
    pub tt_hits: u64,     // probes TT qui ont matché la clé
    pub tt_probes: u64,   // probes TT totales
}






pub struct Search {
    fuel_left: Box<dyn Fn() -> u64>,
    nodes: std::cell::Cell<u32>,
    stopped: std::cell::Cell<bool>,
    zobrist: RefCell<Zobrist>,
    root_ply: Cell<i32>,

    // statistiques
    tt_hits: std::cell::Cell<u64>,      
    tt_probes: std::cell::Cell<u64>,
}



impl Search {
    pub fn new(fuel_left: impl Fn() -> u64 + 'static) -> Search {
        Search { fuel_left: Box::new(fuel_left), 
            nodes: std::cell::Cell::new(0), 
            stopped: std::cell::Cell::new(false), 
            tt_hits: std::cell::Cell::new(0), 
            tt_probes: std::cell::Cell::new(0),
            zobrist: RefCell::new(Zobrist::new()),
            root_ply: Cell::new(0),}
    }

    #[inline]
    fn fuel_left(&self) -> u64 {
        (self.fuel_left)()
    }



    /// Choisit un coup. `seed` = la graine de l'arène : le SEUL hasard autorisé.
    pub fn get_best_move(&self, state: &GameState, utils: &Utils, board: &Board) -> Move {

        let root_maximizing = state.current_player == 0;
        self.nodes.set(0);
        self.stopped.set(false);

        let mut guard = self.zobrist.borrow_mut();
        let zobrist: &mut Zobrist = &mut *guard;
        zobrist.new_search();
        self.root_ply.set(state.ply_count);

        // Iterative deepening
        let mut best_move = Move::pass_mv();
        for depth in 1..=30 {
            if self.stopped.get() { break; }

            // zobrist.clear();
            let (_, mv) = self.alpha_beta(state, utils, board,  depth, i32::MIN, i32::MAX, root_maximizing, zobrist);

            if self.stopped.get() { break; } // itération incomplète
            if mv != Move::pass_mv() {
                best_move = mv;
            }
        }
        best_move

    }



    pub fn alpha_beta(&self, state: &GameState, utils : &Utils, board : &Board, depth: i32, mut alpha: i32, mut beta: i32, maximizing_player: bool, zobrist: &mut Zobrist) -> (i32, Move) {
        if state.is_game_over() || self.stopped.get() {
            return (Evaluator::evaluate(state, utils), Move::pass_mv());
        }
        if depth <= 0 { // Quiescence search
            return (self.quiescence(state, utils, board, alpha, beta, maximizing_player, QSEARCH_MAX_DEPTH),
                    Move::pass_mv());
        }
        
        let original_alpha = alpha;
        let original_beta = beta;
        
        self.tt_probes.set(self.tt_probes.get() + 1); // statistiques : probe TT

        let key = zobrist.get_hash(state);
        let mut _tt_move = Move::pass_mv();
        if let Some(entry) = zobrist.probe(state) {
            self.tt_hits.set(self.tt_hits.get() + 1); // statistiques : probe TT qui match la clé
            _tt_move = entry.best_move; // On mémorise le coup pour le Move Ordering
            if entry.depth >= depth && state.ply_count != self.root_ply.get() {
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

        // Fuel check
        let n = self.nodes.get() + 1;
        self.nodes.set(n);
        if n % 64 == 0 && self.fuel_left() < FUEL_MARGIN {
            self.stopped.set(true);
            return (Evaluator::evaluate(state, utils), Move::pass_mv());
        }

        let mut best_move = Move::pass_mv();
        let mut best_score = if maximizing_player { i32::MIN } else { i32::MAX };

        let mut moves = MoveGenerator::generate_all_moves(state, utils, board); // Todo (prendre en compte le coup du transposition table pour le Move Ordering)
        Search::order_moves(&mut moves, _tt_move, state.current_player as i16);


        if maximizing_player {
            let mut idx = 0usize;

            for mv in moves {
                let new_state = state.apply_move(&mv);

                // Late Move Reductions (LMR)
                // pas de réductions sur les coups de captures, de sortie ou de transposition table, ni sur les 4 premiers coups (les mieux ordonnés)
                let quiet = !mv.captured && !mv.exit && !(mv.from == _tt_move.from && mv.to == _tt_move.to); 
                let reduce = if depth >= LMR_MIN_DEPTH && idx >= LMR_FIRST_MOVE && best_score != i32::MIN && quiet { Search::lmr_reduction(depth, idx) } else { 0 };

                // recherche réduite, si elle bat alpha -> on re-cherche à profondeur complète 
                let eval = if reduce > 0 {
                    let (r, _) = self.alpha_beta(&new_state, utils, board, depth - 1 - reduce, alpha, beta, false, zobrist);
                    if r > alpha {
                        self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, false, zobrist).0
                    } else { r }
                } else {
                    self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, false, zobrist).0
                };

                if eval > best_score { best_score = eval; best_move = mv; }
                alpha = alpha.max(best_score);
                if beta <= alpha { break; } // Beta cutoff
                idx += 1;
            }
        } else {
            let mut idx = 0usize;
            for mv in moves {
                let new_state = state.apply_move(&mv);

                let quiet = !mv.captured && !mv.exit && !(mv.from == _tt_move.from && mv.to == _tt_move.to);
                let reduce = if depth >= LMR_MIN_DEPTH && idx >= LMR_FIRST_MOVE && best_score != i32::MAX && quiet { Search::lmr_reduction(depth, idx) } else { 0 };

                let eval = if reduce > 0 {
                    let (r, _) = self.alpha_beta(&new_state, utils, board, depth - 1 - reduce, alpha, beta, true, zobrist);
                    if r < beta {   // symétrique : la réduite a battu beta → re-cherche
                        self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, true, zobrist).0
                    } else { r }
                } else {
                    self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, true, zobrist).0
                };

                if eval < best_score { best_score = eval; best_move = mv; }
                beta = beta.min(best_score);
                if beta <= alpha { break; } // Alpha cutoff
                idx += 1;
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
            zobrist.add_key(key, depth, best_score, best_move, bound);
        }

        (best_score, best_move)
    }


    pub fn quiescence(&self, state: &GameState, utils: &Utils, board: &Board, mut alpha: i32, mut beta: i32, maximizing_player: bool, qdepth: i32) -> i32 {
        let stand_pat = Evaluator::evaluate(state, utils);

        if state.is_game_over() || self.stopped.get() { return stand_pat; }

        // coupure stand-pat
        if maximizing_player {
            if stand_pat >= beta { return stand_pat; }
            if stand_pat > alpha { alpha = stand_pat; }
        } else {
            if stand_pat <= alpha { return stand_pat; }
            if stand_pat < beta { beta = stand_pat; }
        }
        if qdepth <= 0 { return stand_pat; }


        // Fuel check
        let n = self.nodes.get() + 1;
        self.nodes.set(n);
        if n % 64 == 0 && self.fuel_left() < FUEL_MARGIN {
            self.stopped.set(true);
            return stand_pat;
        }

        // Génération des coups
        let mut moves = MoveGenerator::generate_forcing_moves(state, utils, board);
        if moves.is_empty() { return stand_pat; } // position calme : l'éval statique est fiable

        // sorties et grosses captures en tête (pas de TT en quiescence v1)
        moves.sort_by_key(|mv| std::cmp::Reverse(
            Evaluator::move_ordering_evaluation(mv, Move::pass_mv(), state.current_player as i16)));

        let mut best = stand_pat;
        if maximizing_player {
            for mv in moves {
                let ns = state.apply_move(&mv);
                let score = self.quiescence(&ns, utils, board, alpha, beta, false, qdepth - 1);
                if score > best { best = score; }
                if best > alpha { alpha = best; }
                if alpha >= beta { break; }
            }
        } else {
            for mv in moves {
                let ns = state.apply_move(&mv);
                let score = self.quiescence(&ns, utils, board, alpha, beta, true, qdepth - 1);
                if score < best { best = score; }
                if best < beta { beta = best; }
                if alpha >= beta { break; }
            }
        }
        best
    }




    pub fn order_moves(moves: &mut Vec<Move>, tt_move : Move, player: i16) {
        moves.sort_by_key(|mv| std::cmp::Reverse(Evaluator::move_ordering_evaluation(mv, tt_move, player))); 
    }





    // Late move reductions : 
    #[inline]
    fn lmr_reduction(depth: i32, move_idx: usize) -> i32 {
        let _ = depth;
        if move_idx >= 16 { 2 } else { 1 }
    }




    // statistiques : 
    pub fn get_best_move_with_stats(&self, state: &GameState, utils: &Utils, board: &Board) -> (Move, SearchStats) {
        let root_maximizing = state.current_player == 0;
        self.nodes.set(0);
        self.stopped.set(false);
        self.tt_hits.set(0);
        self.tt_probes.set(0);

        // zobrist
        let mut guard = self.zobrist.borrow_mut();
        let zobrist: &mut Zobrist = &mut *guard;
        zobrist.new_search();
        self.root_ply.set(state.ply_count);

        let mut best_move = Move::pass_mv();
        let mut completed_depth = 0;
        for depth in 1..=30 {
            if self.stopped.get() { break; }
            let (_, mv) = self.alpha_beta(state, utils, board, depth, i32::MIN, i32::MAX, root_maximizing, zobrist);
            if self.stopped.get() { break; } // itération incomplète
            if mv != Move::pass_mv() {
                best_move = mv;
                completed_depth = depth;
            }
        }
        (best_move, SearchStats {
            nodes: self.nodes.get() as u64,
            depth: completed_depth,
            tt_hits: self.tt_hits.get() as u64,
            tt_probes: self.tt_probes.get() as u64,
        })
    }


}