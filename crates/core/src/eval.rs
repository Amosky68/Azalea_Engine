use crate::gamestate::{GameState, Pieces};
use crate::BitboardsUtils::Utils;
use crate::board::Board;
use crate::moves::Move;


// meilleur valeur pour le moment : 180
const RACE_WIN: i32 = 180; // Bonus pour le joueur qui a la sentinel la plus avancée





pub struct Evaluator; 


impl Evaluator {

    /// Choses évaluées 
    /// 
    /// - somme des pièces de chaque joueur
    /// - distance entre la pièce la plus avancée et l'objectif  X  force de la pièce
    /// - le joueur qui a la sentinel la plus avancé à un bonus de
    /// - somme des distances entre toutes les pièces et l'objectif (indépendamment de la force de la pièce)
    /// - score de chaque joueur 
    /// - bonus si sentinel sur les 3 dernières lignes de l'objectif
    /// - bonus pour les pièces alliées qui se touchent 
    /// 


    /// Calcule la distance minimale d'un sentinel à la sortie pour un joueur donné.
    pub fn sentinel_distance_to_exit(state: &GameState, utils: &Utils, player: i32) -> i32 {

        let map = if player == 0 { state.sentinel_p1 } else { state.sentinel_p2 };
        if map == 0 {
            return 10; 
        }
        
        if player == 0 {
            for row in (0..10).rev() {
                let row_mask = utils.get_row_mask(row);
                if map & row_mask != 0 {
                    return 9 - row as i32;
                }
            }
        } else {
            for row in 0..10 {
                let row_mask = utils.get_row_mask(row);
                if map & row_mask != 0 {
                    return row as i32;
                }
            }
        }
        return 10;
    }


    pub fn get_closest_piece_to_goal(state: &GameState, utils : &Utils, player: i32) -> (Pieces, i32) {
        let (ally_explorer, ally_vanguard, ally_sentinel) = state.get_player_pieces_map(player);
        let ally_map = ally_explorer | ally_vanguard | ally_sentinel;
        
        if player == 0 {
            for row in (0..10).rev() {
                let row_mask = utils.get_row_mask(row);
                let pieces_in_row = ally_map & row_mask;
                if pieces_in_row != 0 {
                    if (ally_sentinel & pieces_in_row) != 0 {
                        return (Pieces::Sentinel, 9 - row as i32);
                    } else if (ally_vanguard & pieces_in_row) != 0 {
                        return (Pieces::Vanguard, 9 - row as i32);
                    } else if (ally_explorer & pieces_in_row) != 0 {
                        return (Pieces::Explorer, 9 - row as i32);
                    }
                }
            }
        } else {
            for row in 0..10 {
                let row_mask = utils.get_row_mask(row);
                let pieces_in_row = ally_map & row_mask;
                if pieces_in_row != 0 {
                    if (ally_sentinel & pieces_in_row) != 0 {
                        return (Pieces::Sentinel, row as i32);
                    } else if (ally_vanguard & pieces_in_row) != 0 {
                        return (Pieces::Vanguard, row as i32);
                    } else if (ally_explorer & pieces_in_row) != 0 {
                        return (Pieces::Explorer, row as i32);
                    }
                }
            }
        }
        return (Pieces::None, -1);
    }
    

    pub fn evaluate_from_player_side(state: &GameState, utils : &Utils, player: i32) -> i32 {
        let (ally_explorer, ally_vanguard, ally_sentinel) = state.get_player_pieces_map(player);
        let ally_map = ally_explorer | ally_vanguard | ally_sentinel;
        
        let mut score : i32 = 0;


        // score de chaque joueur 
        let my_score = if player == 0 { state.score_p1 } else { state.score_p2 };
        score += my_score * 1000;


        score += ((ally_explorer.count_ones() + ally_vanguard.count_ones() * 2 + ally_sentinel.count_ones() * 3) as i32) * 10;


        // Distance entre la pièce la plus avancée et l'objectif X force de la pièce
        let (closest_piece, distance) = Evaluator::get_closest_piece_to_goal(state, utils, player);
        if closest_piece != Pieces::None {
            score += (10 - distance) * closest_piece.get_attack_force() as i32 * 40;
        }


        // Somme des distances entre toutes les pièces et l'objectif (indépendamment de la force de la pièce)
        for row in 0..10 {
            let row_mask = utils.get_row_mask(row);
            let ally_pieces_in_row = ally_map & row_mask;

            if ally_pieces_in_row != 0 {
                let ally_piece_count = ally_pieces_in_row.count_ones() as i32;
                let rows_to_go = if player == 0 { 9 - row as i32 } else { row as i32 };
                score += (10 - rows_to_go) * ally_piece_count * 3; 
            }
        }


        // distance de la sentinel la plus avancée à l'objectif
        let sentinel_distance = Evaluator::sentinel_distance_to_exit(state, utils,player);
        score += (10 - sentinel_distance) * 200; 

        // Bonus si sentinel sur les 3 dernières lignes de l'objectif
        let goal_area = if player == 0 { Utils::P1_GOAL_AREA_MASK } else { Utils::P2_GOAL_AREA_MASK };
        if goal_area & ally_sentinel != 0 {
            score += 300;
        }


        let neighboring_map = (ally_map & Utils::TOP_SHIFT_MASK) << 10 | (ally_map & Utils::BOTTOM_SHIFT_MASK) >> 10 | (ally_map & Utils::LEFT_SHIFT_MASK) << 1 | (ally_map & Utils::RIGHT_SHIFT_MASK) >> 1;
        let adjacent_allys = ally_map & neighboring_map;
        score += adjacent_allys.count_ones() as i32 * 10;


        score
    }



    pub fn evaluate(state: &GameState, utils : &Utils) -> i32 {
        // Implémentez votre fonction d'évaluation ici
        // Valeur d'évaluation fictive
        

        if state.is_game_over() {
            let winner = state.get_winner();
            if winner == Some(0) {
                return 1_000_000; // Gagné
            } else if winner == Some(1) {
                return -1_000_000; // Perdu
            }
            return 0; // Match nul
        }


        let player_score = Evaluator::evaluate_from_player_side(state, utils, 0);
        let opponent_score = Evaluator::evaluate_from_player_side(state, utils, 1);

        let mut race_bonus = 0;
        if state.exited_piece_count == 0 {          
            let a = utils.get_best_sentinel_plies(state, 0);
            let b = utils.get_best_sentinel_plies(state, 1);

            let p0_wins_race = if state.current_player == 0 { a <= b } else { a < b };
            race_bonus = if p0_wins_race { RACE_WIN } else { -RACE_WIN };
        }

        return player_score - opponent_score + race_bonus;
    }

    

    pub fn move_ordering_evaluation(mv: &Move,tt_move: Move, player: i16) -> i32 {
        let mut score : i32 = 0;

        if mv.from == tt_move.from && mv.to == tt_move.to {
            score += 10000; // Priorité au coup du transposition table
        }

        if mv.exit {
            score += 1000; // Priorité aux coups qui font sortir une pièce
        }
        if mv.captured {
            score += 500; // Priorité aux coups qui captivent une pièce
        }

        let (_, y1) = Board::index_to_coordinates(mv.from);
        let y2 = if mv.to < 0 { y1 } else { Board::index_to_coordinates(mv.to).1 }; // exit : to = -1
        
        let forward = if player == 0 { y2 > y1 } else { y2 < y1 };
        if forward { score += 100; }

        score += mv.piece.get_attack_force() as i32; // Priorité aux coups avec une force plus élevée
        score
    }


}