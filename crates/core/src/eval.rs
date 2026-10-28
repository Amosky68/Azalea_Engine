use crate::gamestate::{GameState, Pieces};
use crate::board::Board;
use crate::moves::Move;



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




    pub fn has_sentinel_in_goal_area(state: &GameState, player: i32) -> bool {
        let goal_rows = if player == 0 { 7..=9 } else { 0..=2 };
        for index in 0..100 {
            let piece = state.piece_maps[index as usize];
            if piece == Pieces::Sentinel && state.player_piece_map[index as usize] == player as i16 {
                let (_, y) = Board::index_to_coordinates(index);
                if goal_rows.contains(&y) {
                    return true;
                }
            }
        }
        false
    }



    pub fn get_closest_piece_to_goal(state: &GameState, player: i32) -> (Pieces, i32) {
        let mut best = Pieces::None;
        let mut best_key = (i32::MAX, 0i32);
        let mut best_distance = 0i32;

        for index in 0..100i16 {
            if !state.is_occupied_by_ally(index, player as i16) {
                continue;
            }
            let piece = state.piece_maps[index as usize];
            let (_, y) = Board::index_to_coordinates(index);
            let rows_to_go = if player == 0 { 9 - y } else { y };
            let key = (rows_to_go as i32, -(piece.get_attack_force() as i32));

            if key < best_key {
                best_key = key;
                best = piece;
                best_distance = rows_to_go as i32;
            }
        }
        (best, best_distance)
    }



    pub fn evaluate_from_player_side(state: &GameState, player: i32) -> i32 {
        let mut score : i32 = 0;


        // score de chaque joueur 
        score += (state.scores[player as usize] as i32) * 1000;


        for index in 0..100 {
            let piece = state.piece_maps[index as usize];
            if piece != Pieces::None && state.player_piece_map[index as usize] == player as i16 {
                score += piece.get_attack_force() as i32  *  10;
            }
        }

        // Distance entre la pièce la plus avancée et l'objectif X force de la pièce
        let (closest_piece, distance) = Evaluator::get_closest_piece_to_goal(state, player);
        if closest_piece != Pieces::None {
            score += (10 - distance) * closest_piece.get_attack_force() as i32 * 40;
        }

        // Somme des distances entre toutes les pièces et l'objectif (indépendamment de la force de la pièce)
        for index in 0..100 {
            let piece = state.piece_maps[index as usize];
            if piece != Pieces::None && state.player_piece_map[index as usize] == player as i16 {
                let distance = if player == 0 { 9 - Board::index_to_coordinates(index).1 } else { Board::index_to_coordinates(index).1 };
                score += (10 - distance) as i32  *  3;
            }
        }

        // Bonus si sentinel sur les 3 dernières lignes de l'objectif
        if Evaluator::has_sentinel_in_goal_area(state, player) {
            score += 200;
        }

        // Bonus pour les pièces alliées qui se touchent
        for index in 0..100 {
            let piece = state.piece_maps[index as usize];
            if piece != Pieces::None && state.player_piece_map[index as usize] == player as i16 {
                let neighbors = state.board.get_neighbor_indices(index);
                for neighbor in neighbors {
                    if state.is_occupied_by_ally(neighbor, player as i16) {
                        score += 10; // Bonus pour chaque pièce alliée adjacente
                    }
                }
            }
        }

        score
    }



    pub fn evaluate(state: &GameState) -> i32 {
        // Implémentez votre fonction d'évaluation ici
        // Valeur d'évaluation fictive

        if state.is_game_over() {
            let winner = state.get_winner();
            if winner == Some(0) {
                return 1000000; // Gagné
            } else if winner == Some(1) {
                return -1000000; // Perdu
            }
            return 0; // Match nul
        }


        let player_score = Evaluator::evaluate_from_player_side(state, 0);
        let opponent_score = Evaluator::evaluate_from_player_side(state, 1);

        return player_score - opponent_score;
    }

    

    pub fn move_ordering_evaluation(mv: &Move, player: i16) -> i32 {
        let mut score : i32 = 0;

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