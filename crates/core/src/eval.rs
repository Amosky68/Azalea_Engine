use crate::gamestate::{GameState, Pieces};
use crate::bitboardsutils::Utils;
use crate::board::Board;
use crate::moves::Move;


// meilleur valeur pour le moment : 180
const RACE_WIN: i32 = 180; // Bonus pour le joueur qui a la sentinel la plus avancée



const MAT_RACE : [i32; 3] = [150, 250, 700];
const MAT_SCORE: [i32; 3] = [100, 200, 300];

const SCORE_MULTIPLIER: i32 = 1000; 
const CLOSEST_PIECE_MULTIPLIER: i32 = 40; 
const DISTANCE_MULTIPLIER: i32 = 3;
const SENTINEL_DISTANCE_MULTIPLIER: i32 = 200;
const SENTINEL_GOAL_BONUS: i32 = 300;
const ADJACENT_ALLYS_MULTIPLIER: i32 = 10;

const SENTINEL_ALONE_BONUS: i32 = 400; // Bonus si une sentinel est seule (sans enemies) sur une collone de 3 de large
const SENTINEL_NO_ALLY_MALUS: i32 = 0; // Malus si une sentinel n'a pas d'allié à proximité (précalculé dans Utils)

const SENTINEL_THREAT_MALUS: i32 = 250;

const SENTINEL_ESCORT_BONUS: i32 = 80;  // par allié pouvant renforcer en 1 coup
// const SENTINEL_ESCORT_CAP: i32 = 160;   // on compte au max 2 escorts

const ENDGAME_PLY_MALUS: i32 = 150; // Malus par demi-coup pour le joueur qui a la sentinel la plus avancée
const ENDGAME_PIECE_MULTIPLIER: i32 = 300; // Bonus pour le joueur qui a la sentinel la plus avancée






pub struct EvalWeights {
    pub race_win: i32,
    pub score_multiplier: i32,
    pub closest_piece_multiplier: i32,
    pub distance_multiplier: i32,
    pub sentinel_distance_multiplier: i32,
    pub sentinel_goal_bonus: i32,
    pub adjacent_allys_multiplier: i32,
    pub sentinel_alone_bonus: i32,
    pub sentinel_no_ally_malus: i32,
    pub sentinel_threat_malus: i32,
    pub sentinel_escort_bonus: i32,
    // pub sentinel_escort_cap: i32,
    pub endgame_ply_malus: i32,
    pub endgame_piece_multiplier: i32,

    pub mat_race: [i32; 3],
    pub mat_score: [i32; 3],
}


impl EvalWeights {


    pub const NUM_PARAMS: usize = 19;


    pub fn new() -> Self {
        EvalWeights {
            race_win: RACE_WIN,
            score_multiplier: SCORE_MULTIPLIER,
            closest_piece_multiplier: CLOSEST_PIECE_MULTIPLIER,
            distance_multiplier: DISTANCE_MULTIPLIER,
            sentinel_distance_multiplier: SENTINEL_DISTANCE_MULTIPLIER,
            sentinel_goal_bonus: SENTINEL_GOAL_BONUS,
            adjacent_allys_multiplier: ADJACENT_ALLYS_MULTIPLIER,
            sentinel_alone_bonus: SENTINEL_ALONE_BONUS,
            sentinel_no_ally_malus: SENTINEL_NO_ALLY_MALUS,
            sentinel_threat_malus: SENTINEL_THREAT_MALUS,
            sentinel_escort_bonus: SENTINEL_ESCORT_BONUS,
            // sentinel_escort_cap: SENTINEL_ESCORT_CAP,
            endgame_ply_malus: ENDGAME_PLY_MALUS,
            endgame_piece_multiplier: ENDGAME_PIECE_MULTIPLIER,

            mat_race: MAT_RACE,
            mat_score: MAT_SCORE,
        }
    }

    pub fn get_fined_tuned() -> Self { // version à 1930 elo. sans les synergies
        EvalWeights {
            race_win: -19,
            score_multiplier: 571,
            closest_piece_multiplier: -2,
            distance_multiplier: 11,
            sentinel_distance_multiplier: 51,
            sentinel_goal_bonus: 60,
            adjacent_allys_multiplier: -8,
            sentinel_alone_bonus: -5,
            sentinel_no_ally_malus: 7,
            sentinel_threat_malus: 28,
            sentinel_escort_bonus: 3,
            // sentinel_escort_cap: 18,
            endgame_ply_malus: 385,
            endgame_piece_multiplier: 200,

            mat_race: [166, 290, 468],
            mat_score: [214, 395, 325],
        }
    }


    pub fn get_param_mut(&mut self, index: usize) -> &mut i32 {
        match index {
            0 => &mut self.race_win,
            1 => &mut self.score_multiplier,
            2 => &mut self.closest_piece_multiplier,
            3 => &mut self.distance_multiplier,
            4 => &mut self.sentinel_distance_multiplier,
            5 => &mut self.sentinel_goal_bonus,
            6 => &mut self.adjacent_allys_multiplier,
            7 => &mut self.sentinel_alone_bonus,
            8 => &mut self.sentinel_no_ally_malus,
            9 => &mut self.sentinel_threat_malus,
            10 => &mut self.sentinel_escort_bonus,
            11 => &mut self.endgame_ply_malus,
            12 => &mut self.endgame_piece_multiplier,
            // Décomposition des tableaux pour l'optimisation individuelle
            13 => &mut self.mat_race[0],
            14 => &mut self.mat_race[1],
            15 => &mut self.mat_race[2],
            16 => &mut self.mat_score[0],
            17 => &mut self.mat_score[1],
            18 => &mut self.mat_score[2],

            _ => panic!("Index hors limites"),
        }
    }
}





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
    /// 
    ///  Idée d'amélioration : 
    ///  - Bonus si une sentinel est seule sur une collone de 3x3
    ///  -> chaque pièce a une valeur différente (sentinel > vanguard > explorer) plutot que de dire explorer = 1, vanguard = 2, sentinel = 3
    /// 
    ///  - Bonus pour le nombre de cases atteignables si aucune pièce alliée ne bloque le passage (prend en compte la topologie du terrain) (précalculé dans Utils)
    ///  - Malus si un sentinel n'a pas d'allié à proximité (précalculé dans Utils)
    ///  - Précalculer une *zone de préférence* qui est la zone par laquelle les sentinels passent pour aller à l'objectif 
    ///         -> On répètes les itérations bfs depuis le départ de chaque sentinel jusqu'a l'objectif
    ///         -> On pourrait utiliser ces zones qui représente les chemins habituels des sentinels pour l'eval 
    /// 
    /// 
    /// 
    /// 
    ///  - points style RISK pour la hauteur (p1 > p2) alors p1 gagne un bonus 
    ///  -> on fait ça pour chaque pièce qui peut en attaquer une autre 
    ///     --->  on précalcul une height score map (précalculé dans Utils) 
    /// 
    /// 
    /// - Principe de zone d'influences : 
    ///   -> se sont les zones parmis lesquelles les combats vont avoir lieu. 
    ///   -> calcul : ? (idéalement avec des bits maps)
    /// 
    /// - les collones d'influences : 
    ///    -> chaque pièce a une influence sur les collones autour d'elles (1-2-4-2-1) par exemple (multiplié par une constante par pièce)
    ///    -> si l'ennemie se rapproche du milieu/bors du plateau cette influence deviens plus grande (0-1-2-4-5-7-8-9-10-10) / 10
    ///
    ///     ---> Ce que j'ai remarqué : 
    ///         - la quantité d'influence sur une collone est assez représentative du joueur gagnant le combat (Il faudrait adapter cela selon la height map)
    ///         - Lorsque l'enemie a beaucoup d'influence sur une colone, c'est souvent à cet endroit que la sentinelle enemie va passer l'arrivée.
    ///         - il est préférable de regarder loins dans les zones d'influence
    /// 
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
    

    /// Calculates the influence of each corridor on the game state.
    pub fn get_corridor_influence_score(state: &GameState, utils : &Utils, weights: &EvalWeights) -> ([i32; 10], [i32; 10]) {
        let (p0_explorer, p0_vanguard, p0_sentinel) = state.get_player_pieces_map(0);
        let p0_map = p0_explorer | p0_vanguard | p0_sentinel;
        let (p1_explorer, p1_vanguard, p1_sentinel) = state.get_player_pieces_map(1);
        let p1_map = p1_explorer | p1_vanguard | p1_sentinel;

        
        let mut p0_influence = [0;10];
        let mut p1_influence = [0;10];

        let influence_val = [1, 2, 1];
        let vertical_influence_val = [0, 1, 2, 4, 5, 7, 8, 9, 10, 10];
        
        let piece_val = |piece: Pieces| -> i32 {
            match piece {
                Pieces::Explorer => 10,
                Pieces::Vanguard => 20,
                Pieces::Sentinel => 40,
                _ => 0,
            }
        };


        for col in 0..10 {
            let col_mask = utils.get_col_mask(col);
            let mut p0_pieces_in_col = p0_map & col_mask;
            let mut p1_pieces_in_col = p1_map & col_mask;

            // influence joueur 0
            while p0_pieces_in_col != 0 {
                let sq = p0_pieces_in_col.trailing_zeros() as i16;
                let y_mult = vertical_influence_val[(sq / 10) as usize];
                p0_pieces_in_col &= p0_pieces_in_col - 1;

                let piece_value = piece_val(state.get_piece_at(sq)) * y_mult;
                if col != 0 {
                    p0_influence[col as usize - 1] += influence_val[0] * piece_value;
                }
                p0_influence[col as usize] += influence_val[1] * piece_value;
                if col != 9 {
                    p0_influence[col as usize + 1] += influence_val[2] * piece_value;
                }
            }

            // influence joueur 1
            while p1_pieces_in_col != 0 {
                let sq = p1_pieces_in_col.trailing_zeros() as i16;
                let y_mult = vertical_influence_val[((99 - sq) / 10) as usize]; // le joueur 1 est en face
                p1_pieces_in_col &= p1_pieces_in_col - 1;

                let piece_value = piece_val(state.get_piece_at(sq)) * y_mult;
                if col != 0 {
                    p1_influence[col as usize - 1] -= influence_val[0] * piece_value;
                }
                p1_influence[col as usize] -= influence_val[1] * piece_value;
                if col != 9 {
                    p1_influence[col as usize + 1] -= influence_val[2] * piece_value;
                }
            }
        }


        return (p0_influence, p1_influence);
    }



    pub fn evaluate_from_player_side(state: &GameState, utils : &Utils, player: i32, weights: &EvalWeights) -> i32 {
        let (ally_explorer, ally_vanguard, ally_sentinel) = state.get_player_pieces_map(player);
        let ally_map = ally_explorer | ally_vanguard | ally_sentinel;
        let (enemy_explorer, enemy_vanguard, enemy_sentinel) = state.get_player_pieces_map(1 - player);
        let enemy_map = enemy_explorer | enemy_vanguard | enemy_sentinel;
        
        let mut score : i32 = 0;


        // score de chaque joueur 
        let my_score = if player == 0 { state.score_p1 } else { state.score_p2 };
        score += my_score * weights.score_multiplier;


        // score += ((ally_explorer.count_ones() + ally_vanguard.count_ones() * 2 + ally_sentinel.count_ones() * 3) as i32) * 10;
        let race_alive = state.exited_piece_count == 0 && (state.sentinel_p1 | state.sentinel_p2) != 0;
        let values = if race_alive { &weights.mat_race } else { &weights.mat_score };
        score += ally_explorer.count_ones() as i32 * values[0] + ally_vanguard.count_ones() as i32 * values[1] + ally_sentinel.count_ones() as i32 * values[2];


        // Distance entre la pièce la plus avancée et l'objectif X force de la pièce
        let (closest_piece, distance) = Evaluator::get_closest_piece_to_goal(state, utils, player);
        if closest_piece != Pieces::None {
            score += (10 - distance) * closest_piece.get_attack_force() as i32 * weights.closest_piece_multiplier;
        }


        // Somme des distances entre toutes les pièces et l'objectif (indépendamment de la force de la pièce)
        for row in 0..10 {
            let row_mask = utils.get_row_mask(row);
            let ally_pieces_in_row = ally_map & row_mask;

            if ally_pieces_in_row != 0 {
                let ally_piece_count = ally_pieces_in_row.count_ones() as i32;
                let rows_to_go = if player == 0 { 9 - row as i32 } else { row as i32 };
                score += (10 - rows_to_go) * ally_piece_count * weights.distance_multiplier; 
            }
        }


        // distance de la sentinel la plus avancée à l'objectif
        let sentinel_distance = Evaluator::sentinel_distance_to_exit(state, utils,player);
        score += (10 - sentinel_distance) * weights.sentinel_distance_multiplier; 

        // Bonus si sentinel sur les 3 dernières lignes de l'objectif
        let goal_area = if player == 0 { Utils::P1_GOAL_AREA_MASK } else { Utils::P2_GOAL_AREA_MASK };
        if goal_area & ally_sentinel != 0 {
            score += weights.sentinel_goal_bonus;
        }


        // Bonus si une sentinel est seule (sans enemies) sur une collone de 3 de large 
        let sentinel_1 = ally_sentinel & (ally_sentinel - 1);
        let sentinel_2 = ally_sentinel ^ sentinel_1;

        if sentinel_1 != 0 {
            let sentinel_1_index = sentinel_1.trailing_zeros() as usize;
            score += ((enemy_map & utils.get_triple_col_mask(sentinel_1_index % 10)) == 0) as i32 * weights.sentinel_alone_bonus;
            score -= ((utils.get_free_movement_layout(4, sentinel_1_index) & ally_map) == 0) as i32 * weights.sentinel_no_ally_malus;
        }

        if sentinel_2 != 0 {
            let sentinel_2_index = sentinel_2.trailing_zeros() as usize;
            score += ((enemy_map & utils.get_triple_col_mask(sentinel_2_index % 10)) == 0) as i32 * weights.sentinel_alone_bonus;
            score -= ((utils.get_free_movement_layout(4, sentinel_2_index) & ally_map) == 0) as i32 * weights.sentinel_no_ally_malus;
        }


        // Alliées venant escoter la sentinelle leader 
        if ally_sentinel != 0 {
            let mut leader_sq = 0;
            let mut min_dist = 100;
            let mut temp_s = ally_sentinel;
            
            // Recherche de la sentinelle leader (la plus proche de la ligne de sortie)
            while temp_s != 0 {
                let sq = temp_s.trailing_zeros() as usize;
                temp_s &= temp_s - 1;
                let dist = if player == 0 { 9 - (sq / 10) } else { sq / 10 };
                if dist < min_dist {
                    min_dist = dist;
                    leader_sq = sq;
                }
            }

            let s_neighbors = utils.get_neighbor_mask(leader_sq as i16);

            // R1 — Menace sur la sentinelle leader (phase de course uniquement)
            if state.exited_piece_count == 0 {
                let support = s_neighbors & ally_map; // Allié adjacent à la sentinelle leader
                let support_count = (s_neighbors & ally_map).count_ones() as i32;

                if support == 0 {
                    let mut hunter_reach = 0_u128;

                    // Portée des sentinelles ennemies (budget 3)
                    let mut es = enemy_sentinel;
                    while es != 0 {
                        let e = es.trailing_zeros() as usize;
                        es &= es - 1;
                        hunter_reach |= utils.get_free_movement_layout(3, e);
                    }

                    // Portée des avant-gardes ennemies (budget 4)
                    let mut ev = enemy_vanguard;
                    while ev != 0 {
                        let e = ev.trailing_zeros() as usize;
                        ev &= ev - 1;
                        hunter_reach |= utils.get_free_movement_layout(4, e);
                    }

                    // Si un chasseur ennemi peut atteindre une case adjacente sans soutien allié
                    if hunter_reach & s_neighbors != 0 {
                        if support_count == 0 {
                            // R1 (inchangé) : n'importe quel S/V seul prend le runner (3 = 3)
                            score -= weights.sentinel_threat_malus;
                        } else {
                            // C2 (nouveau) : paire chasseur + explorer (attaque 3+1 = 4) bat la défense 3+1
                            let mut explorer_reach = 0_u128;
                            let mut ee = enemy_explorer;
                            while ee != 0 {
                                let e = ee.trailing_zeros() as usize;
                                ee &= ee - 1;
                                explorer_reach |= utils.get_free_movement_layout(5, e);
                            }
                            if explorer_reach & s_neighbors != 0 {
                                score -= weights.sentinel_threat_malus;
                            }
                        }
                    }
                }
            }

            // Calcul du nombre d'escortes capables d'atteindre le voisinage de la sentinelle leader
            let mut escorts = 0;
            let mut temp = ally_map & !(1u128 << leader_sq);
            while temp != 0 {
                let sq = temp.trailing_zeros() as usize;
                temp &= temp - 1;
                
                let mask = 1u128 << sq;
                let b = if ally_sentinel & mask != 0 {
                    3
                } else if ally_vanguard & mask != 0 {
                    4
                } else {
                    5
                };

                if utils.get_free_movement_layout(b, sq) & s_neighbors != 0 {
                    escorts += 1;
                }
            }
            score += escorts.min(2) * weights.sentinel_escort_bonus;
        }


        // de combien on gagne la course finale 
        if state.exited_piece_count == 1 {
            let mut map = ally_map;
            let mut min_dist = 100;
            let mut piece = Pieces::None;
            while map != 0 {
                let sq = map.trailing_zeros() as usize;
                map &= map - 1;
                let dist = utils.get_distance_to_exit(player as usize, sq);

                if dist < min_dist {
                    min_dist = dist;
                    piece = state.get_piece_at(sq as i16);
                }
            }
            if  piece != Pieces::None  {
                let budget = piece.get_movement_budget();
                let my_plies = (min_dist as i32 + budget) / budget;

                score += - weights.endgame_ply_malus * my_plies + piece.get_attack_force() as i32 * weights.endgame_piece_multiplier;
            }
        }


        let neighboring_map = (ally_map & Utils::TOP_SHIFT_MASK) << 10 | (ally_map & Utils::BOTTOM_SHIFT_MASK) >> 10 | (ally_map & Utils::LEFT_SHIFT_MASK) << 1 | (ally_map & Utils::RIGHT_SHIFT_MASK) >> 1;
        let adjacent_allys = ally_map & neighboring_map;
        score += adjacent_allys.count_ones() as i32 * weights.adjacent_allys_multiplier;


        score
    }




    pub fn evaluate(state: &GameState, utils : &Utils, weights: &EvalWeights) -> i32 {
        // Implémentez votre fonction d'évaluation ici
        // Valeur d'évaluation fictive
        

        if state.is_game_over() {
            let winner = state.get_winner();
            if winner == Some(0) {
                return 1_000_000 - state.ply_count; // Gagné
            } else if winner == Some(1) {
                return -1_000_000 + state.ply_count; // Perdu
            }
            return 0; // Match nul
        }



        let player_score = Evaluator::evaluate_from_player_side(state, utils,  0, weights);
        let opponent_score = Evaluator::evaluate_from_player_side(state, utils, 1, weights);

        let mut race_bonus = 0;
        if state.exited_piece_count == 0 {          
            let a = utils.get_best_sentinel_plies(state, 0);
            let b = utils.get_best_sentinel_plies(state, 1);

            let p0_wins_race = if state.current_player == 0 { a <= b } else { a < b };
            race_bonus = if p0_wins_race { weights.race_win } else { -weights.race_win };
        }


        return player_score - opponent_score + race_bonus;
    }

    

    pub fn move_ordering_evaluation(mv: &Move, tt_move: &Move, killer_1 : &Move, killer_2 : &Move, player: i16) -> i32 {
        let mut score : i32 = 0;

        if mv.from == tt_move.from && mv.to == tt_move.to {
            score += 10000; // Priorité au coup du transposition table
        }

        if (mv.from == killer_1.from && mv.to == killer_1.to) || (mv.from == killer_2.from && mv.to == killer_2.to) {
            score += 5000; // Priorité aux coups killer
        }

        if mv.exit {
            score += 1000; // Priorité aux coups qui font sortir une pièce
        }
        if mv.captured {
            score += 500; // Priorité aux coups qui captivent une pièce
        }
        if mv.piece == Pieces::Sentinel {
            score += 200; // Priorité aux coups qui déplacent une sentinel
        }

        let (_, y1) = Board::index_to_coordinates(mv.from);
        let y2 = if mv.to < 0 { y1 } else { Board::index_to_coordinates(mv.to).1 }; // exit : to = -1
        
        let forward = if player == 0 { y2 > y1 } else { y2 < y1 };
        if forward { score += 100; }



        score += mv.piece.get_attack_force() as i32; // Priorité aux coups avec une force plus élevée
        score
    }


}