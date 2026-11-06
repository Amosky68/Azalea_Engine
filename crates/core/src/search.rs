use crate::gamestate::{GameState, Pieces};
use crate::moves::{Move, MoveGenerator};
use crate::eval::{Evaluator, EvalWeights};
use crate::zobrist::{Zobrist, Bounds};
use crate::bitboardsutils::Utils;
use crate::board::Board;
use core::num;
use std::cell::{Cell, RefCell};



const FUEL_MARGIN : u64 = 100_000_000;
const QSEARCH_MAX_DEPTH: i32 = 4;

const LMR_MIN_DEPTH : i32  = 2;  // jamais de réduction au-dessus de l'horizon rapproché
const LMR_FIRST_MOVE: usize = 6; // jamais sur les 4 premiers coups (les mieux ordonnés)
const NMP_MIN_DEPTH : i32  = 3;  // jamais de Null Move Pruning au-dessus de l'horizon rapproché

const ITERATIVE_DEEPENING_MAX_DEPTH : usize = 30; // profondeur max de recherche itérative




#[derive(Debug, Clone, Copy, Default)]
pub struct SearchStats {
    pub nodes: u64,       // nœuds visités ce coup
    pub depth: i32,       // dernière profondeur COMPLÈTE
    pub tt_hits: u64,     // probes TT qui ont matché la clé
    pub tt_probes: u64,   // probes TT totales
}




pub struct Search {
    fuel_left: Box<dyn Fn() -> u64>,
    nodes: u32,
    stopped: bool,
    zobrist: Zobrist,
    root_ply: i32,
    weight: EvalWeights, // poids d'évaluation pour la recherche
    pub max_depth: usize, // profondeur max de l'ID (tests/bench ; 29 = comportement arène)

    move_buffer: [[Move; 256]; ITERATIVE_DEEPENING_MAX_DEPTH],
    move_buffer_index : [usize; ITERATIVE_DEEPENING_MAX_DEPTH],

    forcing_moves: [[Move; 256]; ITERATIVE_DEEPENING_MAX_DEPTH],
    forcing_moves_index : [usize; ITERATIVE_DEEPENING_MAX_DEPTH],

    killer_moves: [[Move; 2]; 30],

    // statistiques
    tt_hits: u64,      
    tt_probes: u64,
}



impl Search {
    pub fn new(fuel_left: impl Fn() -> u64 + 'static) -> Search {
        Search { fuel_left: Box::new(fuel_left), 
            nodes: 0, 
            stopped: false, 
            tt_hits: 0, 
            tt_probes: 0,
            zobrist: Zobrist::new(),
            root_ply: 0,
            weight: EvalWeights::get_fined_tuned(),
            max_depth: ITERATIVE_DEEPENING_MAX_DEPTH - 1,

            move_buffer: [[Move::pass_mv(); 256]; ITERATIVE_DEEPENING_MAX_DEPTH],
            move_buffer_index : [0; ITERATIVE_DEEPENING_MAX_DEPTH],

            forcing_moves: [[Move::pass_mv(); 256]; ITERATIVE_DEEPENING_MAX_DEPTH],
            forcing_moves_index : [0; ITERATIVE_DEEPENING_MAX_DEPTH],

            killer_moves: [[Move::pass_mv(); 2]; 30],


            // Inclut les données JSON directement dans le binaire .wasm à la compilation
            // synergy_maps: SynergyTables::load_from_str(include_str!("synergy_maps.json")).expect("Impossible de parser synergy_maps.json embarqué"),
        }
    }

    #[inline]
    fn fuel_left(&self) -> u64 {
        (self.fuel_left)()
    }



    /// Choisit un coup. `seed` = la graine de l'arène : le SEUL hasard autorisé.
    pub fn get_best_move(&mut self, state: &GameState, utils: &Utils, board: &Board) -> Move {
        let root_maximizing = state.current_player == 0;
        self.nodes = 0;
        self.stopped = false;

        self.zobrist.new_search();
        self.root_ply = state.ply_count;


        // Iterative deepening
        let mut best_move = Move::pass_mv();
        for depth in 1..=self.max_depth.min(ITERATIVE_DEEPENING_MAX_DEPTH - 1) {
            if self.stopped { break; }

            // zobrist.clear();
            let (_, mv) = self.alpha_beta(state, utils, board,  depth as i32, i32::MIN, i32::MAX, root_maximizing);

            if self.stopped { break; } // itération incomplète
            if mv != Move::pass_mv() {
                best_move = mv;
            }
        }
        best_move

    }



    pub fn alpha_beta(&mut self, state: &GameState, utils : &Utils, board : &Board, depth: i32, mut alpha: i32, mut beta: i32, maximizing_player: bool) -> (i32, Move) {
        if state.is_game_over() || self.stopped {
            return (Evaluator::evaluate(state, utils,  &self.weight), Move::pass_mv());
        }
        if depth <= 0 { // Quiescence search
            return (self.quiescence(state, utils, board, alpha, beta, maximizing_player, QSEARCH_MAX_DEPTH),
                    Move::pass_mv());
        }
        
        let original_alpha = alpha;
        let original_beta = beta;
        
        self.tt_probes = self.tt_probes + 1; // statistiques : probe TT

        let key = self.zobrist.get_hash(state);
        let mut _tt_move = Move::pass_mv();
        if let Some(entry) = self.zobrist.probe(state) {
            self.tt_hits = self.tt_hits + 1; // statistiques : probe TT qui match la clé
            _tt_move = entry.best_move; // On mémorise le coup pour le Move Ordering
            if entry.depth >= depth && state.ply_count != self.root_ply {
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
        let n = self.nodes + 1;
        self.nodes = n;
        if n % 64 == 0 && self.fuel_left() < FUEL_MARGIN {
            self.stopped = true;
            return (Evaluator::evaluate(state, utils,  &self.weight), Move::pass_mv());
        }


        // Null Move Pruning (NMP)
        // On ne le fait pas en fin de partie, ni à très faible profondeur, ni si le coup précédent était déjà un Null Move
        let is_endgame = state.exited_piece_count > 0;
        if depth >= NMP_MIN_DEPTH && !is_endgame {
            let r = if depth > 6 { 3 } else { 2 }; // Réduction de profondeur
            let null_state = state.apply_move(&Move::pass_mv());
            
            if maximizing_player {
                let (null_score, _) = self.alpha_beta(&null_state, utils, board, depth - 1 - r, beta - 1, beta, false);
                if null_score >= beta {
                    return (beta, Move::pass_mv()); // Coupure ! On gagne une branche entière.
                }
            } else {
                let (null_score, _) = self.alpha_beta(&null_state, utils, board, depth - 1 - r, alpha, alpha + 1, true);
                if null_score <= alpha {
                    return (alpha, Move::pass_mv()); // Coupure !
                }
            }
        }



        let mut best_move = Move::pass_mv();
        let mut best_score = if maximizing_player { i32::MIN } else { i32::MAX };


        MoveGenerator::generate_all_moves(state, utils, board, &mut self.move_buffer[depth as usize], &mut self.move_buffer_index[depth as usize]);
        self.order_moves( _tt_move, depth as usize, state.current_player as i16);

        let influence_zone = state.get_influce_zone(utils);


        if maximizing_player {
            for idx in 0..self.move_buffer_index[depth as usize] {
                let mv = self.move_buffer[depth as usize][idx];
                let new_state = state.apply_move(&mv);

                // Late Move Reductions (LMR)
                // pas de réductions sur les coups de captures, de sortie ou de transposition table, ni sur les 4 premiers coups (les mieux ordonnés)
                let quiet = !mv.captured && !mv.exit && !(mv.from == _tt_move.from && mv.to == _tt_move.to) && !(mv.piece == Pieces::Sentinel && state.exited_piece_count == 0); 
                let reduce = if depth >= LMR_MIN_DEPTH && idx >= LMR_FIRST_MOVE && best_score != i32::MIN && quiet { Search::lmr_reduction(depth, idx, influence_zone, &mv) } else { 0 };

                // recherche réduite, si elle bat alpha -> on re-cherche à profondeur complète 
                let eval = if reduce > 0 {
                    let (r, _) = self.alpha_beta(&new_state, utils, board, depth - 1 - reduce, alpha, beta, false);
                    if r > alpha {
                        self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, false).0
                    } else { r }
                } else {
                    self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, false).0
                };

                if eval > best_score { best_score = eval; best_move = mv; }
                alpha = alpha.max(best_score);
                if beta <= alpha { 
                    if !mv.captured && !mv.exit && self.killer_moves[depth as usize][0] != mv {
                        // On décale le killer 1 vers le killer 2, et on insère le nouveau en 1
                        self.killer_moves[depth as usize][1] = self.killer_moves[depth as usize][0];
                        self.killer_moves[depth as usize][0] = mv;
                    }
                    break;
                } // Beta cutoff
                
            }
        } else {
            for idx in 0..self.move_buffer_index[depth as usize] {
                let mv = self.move_buffer[depth as usize][idx];
                let new_state = state.apply_move(&mv);

                // Late Move Reductions (LMR)
                // pas de réductions sur les coups de captures, de sortie ou de transposition table, ni sur les 4 premiers coups (les mieux ordonnés)
                let quiet = !mv.captured && !mv.exit && !(mv.from == _tt_move.from && mv.to == _tt_move.to) && !(mv.piece == Pieces::Sentinel && state.exited_piece_count == 0);
                let reduce = if depth >= LMR_MIN_DEPTH && idx >= LMR_FIRST_MOVE && best_score != i32::MAX && quiet { Search::lmr_reduction(depth, idx, influence_zone, &mv) } else { 0 };

                let eval = if reduce > 0 {
                    let (r, _) = self.alpha_beta(&new_state, utils, board, depth - 1 - reduce, alpha, beta, true);
                    if r < beta {   // symétrique : la réduite a battu beta → re-cherche
                        self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, true).0
                    } else { r }
                } else {
                    self.alpha_beta(&new_state, utils, board, depth - 1, alpha, beta, true).0
                };

                if eval < best_score { best_score = eval; best_move = mv; }
                beta = beta.min(best_score);
                if beta <= alpha { 
                    if !mv.captured && !mv.exit && self.killer_moves[depth as usize][0] != mv {
                        self.killer_moves[depth as usize][1] = self.killer_moves[depth as usize][0];
                        self.killer_moves[depth as usize][0] = mv;
                    }
                    break;
                } // Alpha cutoff
            }
        }

        // Store in transposition table
        if !self.stopped {
            let bound = if best_score <= original_alpha {
                Bounds::Upper
            } else if best_score >= original_beta {
                Bounds::Lower
            } else {
                Bounds::Exact
            };
            self.zobrist.add_key(key, depth, best_score, best_move, bound);
        }

        (best_score, best_move)
    }


    pub fn quiescence(&mut self, state: &GameState, utils: &Utils, board: &Board, mut alpha: i32, mut beta: i32, maximizing_player: bool, qdepth: i32) -> i32 {
        let stand_pat = Evaluator::evaluate(state, utils,  &self.weight);

        if state.is_game_over() || self.stopped { return stand_pat; }

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
        let n = self.nodes + 1;
        self.nodes = n;
        if n % 64 == 0 && self.fuel_left() < FUEL_MARGIN {
            self.stopped = true;
            return stand_pat;
        }

        // Génération des coups
        MoveGenerator::generate_forcing_moves(state, utils, board, &mut self.forcing_moves[qdepth as usize], &mut self.forcing_moves_index[qdepth as usize]);

        let q_idx = qdepth as usize;
        let num_moves = self.forcing_moves_index[q_idx];
        if num_moves == 0 { return stand_pat; } // position calme : l'éval statique est fiable

        // sorties et grosses captures en tête (pas de TT en quiescence v1)
        self.forcing_moves[qdepth as usize][..num_moves].sort_unstable_by_key(|mv| std::cmp::Reverse(
            Evaluator::move_ordering_evaluation(mv, &Move::pass_mv(), &self.killer_moves[q_idx][0], &self.killer_moves[q_idx][1], state.current_player as i16)
        ));



        let mut best = stand_pat;
        if maximizing_player {
            for idx in 0..num_moves {
                let mv = self.forcing_moves[q_idx][idx];
                let ns = state.apply_move(&mv);
                let score = self.quiescence(&ns, utils, board, alpha, beta, false, qdepth - 1);
                if score > best { best = score; }
                if best > alpha { alpha = best; }
                if alpha >= beta { break; }
            }
        } else {
            for idx in 0..num_moves {
                let mv = self.forcing_moves[q_idx][idx];
                let ns = state.apply_move(&mv);
                let score = self.quiescence(&ns, utils, board, alpha, beta, true, qdepth - 1);
                if score < best { best = score; }
                if best < beta { beta = best; }
                if alpha >= beta { break; }
            }
        }
        best
    }


    pub fn order_moves(&mut self, tt_move: Move, depth : usize, player : i16) {
        let moves = &mut self.move_buffer[depth as usize][..self.move_buffer_index[depth as usize]];
        moves.sort_unstable_by_key(|mv| std::cmp::Reverse(
            Evaluator::move_ordering_evaluation(mv, &tt_move, &self.killer_moves[depth][0], &self.killer_moves[depth][1], player)
        )); 
    }

    

    // Late move reductions : 
    #[inline]
    fn lmr_reduction(_depth: i32, move_idx: usize, influence_zone : u128, mv : &Move) -> i32 {
        
        let mut reduction = 0;

        if move_idx >= 8 { 
            reduction += 2; 
        } else { 
            reduction += 1; 
        }

        reduction
    }




    // statistiques : 
    pub fn get_best_move_with_stats(&mut self, state: &GameState, utils: &Utils, board: &Board) -> (Move, SearchStats) {
        let root_maximizing = state.current_player == 0;
        self.nodes = 0;
        self.stopped= false;
        self.tt_hits = 0;
        self.tt_probes = 0;

        // zobrist
        self.zobrist.new_search();
        self.root_ply = state.ply_count;

        let mut best_move = Move::pass_mv();
        let mut completed_depth = 0;
        for depth in 1..=self.max_depth.min(ITERATIVE_DEEPENING_MAX_DEPTH - 1) {
            if self.stopped { break; }
            let (_, mv) = self.alpha_beta(state, utils, board, depth as i32, i32::MIN, i32::MAX, root_maximizing);
            if self.stopped { break; } // itération incomplète
            if mv != Move::pass_mv() {
                best_move = mv;
                completed_depth = depth;
            }
        }
        (best_move, SearchStats {
            nodes: self.nodes as u64,
            depth: completed_depth as i32,
            tt_hits: self.tt_hits as u64,
            tt_probes: self.tt_probes as u64,
        })
    }


}