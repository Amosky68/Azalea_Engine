use crate::gamestate::GameState;
use crate::gamestate::Pieces;
use crate::BitboardsUtils::Utils;
use crate::board::Board;




#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move {
    pub from: i16, // indice de la case de départ 
    pub to: i16,   // indice de la case d'arrivée
    pub piece: Pieces, // pièce qui est déplacée

    //pub piece: Piece, // pièce qui est déplacée

    pub captured_piece: Pieces,
    pub captured: bool,
    pub exit: bool, // true si le mouvement est une sortie de pièce du plateau 
}

impl Move {
    pub fn pass_mv() -> Move {
        Move {
            from: -1,
            to: -1,
            piece: Pieces::None,
            captured_piece: Pieces::None,
            captured: false,
            exit: false,
        }
    }

    pub fn is_pass(&self) -> bool {
        self.from == -1 && self.to == -1
    }
}



pub struct MoveGenerator {

}

impl MoveGenerator {


    pub fn can_piece_attack(gs: &GameState, piece: Pieces, from : i16, to : i16, vanguard_attack_bonus: bool, utils : &Utils, board : &Board) -> bool {
        let mut piece_allied_force = piece.get_attack_force();

        piece_allied_force += if board.get_height(from) > board.get_height(to) { 1 } else { 0 }; 
        piece_allied_force += if board.get_height(from) < board.get_height(to) { -1 } else { 0 }; 

        let (ally_explorer, ally_vanguard, ally_sentinel) = gs.get_ally_pieces_masks();
        let (_, enemy_mask) = gs.get_ally_ennemy_masks();

        let neighbor_map = utils.get_neighbor_mask(to) & (!(1_u128 << from)); // mask des voisins de la case cible, en excluant la case de départ
        piece_allied_force += (neighbor_map & ally_explorer).count_ones() as i32;
        piece_allied_force += (neighbor_map & ally_vanguard).count_ones() as i32 * 2;
        piece_allied_force += (neighbor_map & ally_sentinel).count_ones() as i32 * 3;


        // charge de l'avant garde : si l'attaquante est une Avant-garde et si le coût total de l'attaque est EXACTEMENT 4, attaque += 1.
        if vanguard_attack_bonus {
            piece_allied_force += 1;
        }

        let mut piece_enemy_force = gs.get_piece_at(to).get_attack_force();

        piece_enemy_force += (utils.get_neighbor_mask(to) & enemy_mask).count_ones() as i32; // mask des voisins de la case cible, en excluant la case de départ

        piece_allied_force >= piece_enemy_force
    }

    /// renvoie (free moves, attack moves)
    pub fn get_explorer_special_moves(gs: &GameState, index: i16, reached_squares: u128, utils : &Utils, board : &Board) -> (u128, u128) {
        let (ally_mask, ennemy_mask) = gs.get_ally_ennemy_masks();
        let all_occupied = ally_mask | ennemy_mask;


        let diag_neighbor_mask = utils.get_diagonal_neighbor_mask(index) & !reached_squares & !ally_mask;
        let mut diag_ennemy_occupied = diag_neighbor_mask & ennemy_mask;
        let mut free_moves = diag_neighbor_mask & !ennemy_mask; // cases forcément libres 
        let mut attack_moves = 0_u128;


        while diag_ennemy_occupied != 0 {
            let ennemy_index = diag_ennemy_occupied.trailing_zeros();
            diag_ennemy_occupied ^= 1_u128 << ennemy_index; // retire l'ennemi trouvé du masque

            if MoveGenerator::can_piece_attack(gs, Pieces::Explorer, index, ennemy_index as i16, false, utils, board) {
                attack_moves |= 1_u128 << ennemy_index;
            }
        }

        let mut potential_leaps = utils.get_two_step_mask(index) & !reached_squares & !ally_mask;


        while potential_leaps != 0 {
            let dest_index = potential_leaps.trailing_zeros();
            potential_leaps ^= 1_u128 << dest_index; // Dépiler le bit traité


            let between_index = (index as u32 + dest_index) / 2;
            let between_mask = 1_u128 << between_index;

            // La case intermédiaire doit être vide
            if (all_occupied & between_mask) == 0 {
                let dest_mask = 1_u128 << dest_index;

                if (ennemy_mask & dest_mask) != 0 { // occupation ennemie
                    if MoveGenerator::can_piece_attack(gs, Pieces::Explorer, index, dest_index as i16, false, utils, board) {
                        attack_moves |= dest_mask;
                    }
                } else {
                    free_moves |= dest_mask;
                }
            }
        }
        
        (free_moves, attack_moves)
    }


    
    /// Returns a list of all squares that can be reached by the given piece.
    pub fn get_piece_moves(gs: &GameState, index: i16, piece: Pieces, utils : &Utils, board : &Board) -> (u128, u128, bool) {
        // Bitboard bfs 

        let (ally_mask, ennemy_mask) = gs.get_ally_ennemy_masks();
        let piece_budget = piece.get_movement_budget();

        let mut budget = piece_budget;
        let mut reach = [0u128; 6];
        reach[budget as usize] = 1_u128 << index;

        let empty_mask = !(ally_mask | ennemy_mask) ;
        let mut mask = reach[budget as usize];

        // attaques ennemis 
        let mut sucessfull_attaks = 0_u128; // attaques qui réussissent
        let mut attempted_attaks  = 0_u128; // attaques que l'on a tenté (pour ne pas répéter une attaque avec un autre cost)


        while  budget > 0 {
            if budget > 0 {
                let next_step = (mask & utils.get_bfs_weight(0, 0)) << 10 // up
                    | (mask & utils.get_bfs_weight(0, 1)) << 1   // right
                    | (mask & utils.get_bfs_weight(0, 2)) >> 10  // down
                    | (mask & utils.get_bfs_weight(0, 3)) >> 1;  // left  

                reach[budget as usize - 1] |= next_step;
            }

            if budget > 1 {
                let next_step = (mask & utils.get_bfs_weight(1, 0)) << 10 
                    | (mask & utils.get_bfs_weight(1, 1)) << 1 
                    | (mask & utils.get_bfs_weight(1, 2)) >> 10 
                    | (mask & utils.get_bfs_weight(1, 3)) >> 1; 

                reach[budget as usize - 2] |= next_step;
            }

            if budget > 2 {
                let next_step = (mask & utils.get_bfs_weight(2, 0)) << 10 
                    | (mask & utils.get_bfs_weight(2, 1)) << 1 
                    | (mask & utils.get_bfs_weight(2, 2)) >> 10 
                    | (mask & utils.get_bfs_weight(2, 3)) >> 1; 

                reach[budget as usize - 3] |= next_step;
            }

            budget -= 1;

            let mut attacks_to_test = reach[budget as usize] & ennemy_mask & !attempted_attaks;
            attempted_attaks |= attacks_to_test;

            while attacks_to_test != 0 {
                let attack_id = attacks_to_test.trailing_zeros() as i16;
                attacks_to_test &= attacks_to_test - 1;

                let vanguard_attack_bonus = piece == Pieces::Vanguard && budget == 0;

                let can_attack = MoveGenerator::can_piece_attack(gs, piece, index, attack_id, vanguard_attack_bonus, utils, board);
                if can_attack {
                    sucessfull_attaks |= 1_u128 << attack_id;
                }
            }

            mask = reach[budget as usize] & empty_mask;
        }

        let all_reach_except_0 = reach[1] | reach[2] | reach[3] | reach[4] | reach[5];
        let all_reach = all_reach_except_0 | reach[0];

        let exit_mask = if gs.current_player == 0 { Utils::P1_EXIT_MASK } else { Utils::P2_EXIT_MASK };
        let can_exit = (all_reach_except_0 & exit_mask & (empty_mask | (1_u128 << index))) != 0;

        let (explorer_free_moves, explorer_attack_moves) = 
            if piece == Pieces::Explorer { 
                MoveGenerator::get_explorer_special_moves(gs, index, (all_reach & empty_mask) | sucessfull_attaks, utils, board) 
            } else { 
                (0_u128, 0_u128) 
            };

        let free_moves = (all_reach & empty_mask & !(1_u128 << index)) | explorer_free_moves;
        let attack_moves = sucessfull_attaks | explorer_attack_moves;        

        return (free_moves, attack_moves, can_exit);
    }

     
    pub fn generate_all_moves(gs: &GameState, utils : &Utils, board : &Board, moves: &mut Vec<Move>) {
        moves.clear();

        let player_maps = if gs.current_player == 0 { [gs.explorer_p1, gs.vanguard_p1, gs.sentinel_p1] } else { [gs.explorer_p2, gs.vanguard_p2, gs.sentinel_p2] };
        let pieces = [Pieces::Explorer, Pieces::Vanguard, Pieces::Sentinel];


        for i in 0..3  {

            let mut map = player_maps[i];
            let piece = pieces[i];
            while map != 0 {
                let index = map.trailing_zeros() as i16;
                map &= map - 1;

                let (mut free_moves, mut attack_moves, can_exit) = MoveGenerator::get_piece_moves(gs, index, piece, utils, board);

                if can_exit {
                    moves.push(Move {from: index, to: -1, piece: piece, captured_piece: Pieces::None, captured: false, exit: true});
                }

                while attack_moves != 0 {
                    let to = attack_moves.trailing_zeros() as i16;
                    attack_moves &= attack_moves - 1;

                    let captured_piece = gs.get_piece_at(to);

                    moves.push(Move {from: index, to: to, piece: piece, captured_piece: captured_piece, captured: true, exit: false} );
                }

                while free_moves != 0 {
                    let to = free_moves.trailing_zeros() as i16;
                    free_moves &= free_moves - 1;

                    moves.push(Move {from: index, to: to, piece: piece, captured_piece: Pieces::None, captured: false, exit: false} );
                }

            }
        }


        
        if moves.is_empty() { // le move "pass"
            moves.push(Move {
                from: -1,
                to: -1,
                piece: Pieces::None,
                captured_piece: Pieces::None,
                captured: false,
                exit: false,
            });
        }

    }


    /// Utile pour la recherche quiescence
    pub fn generate_forcing_moves(gs: &GameState, utils: &Utils, board: &Board) -> Vec<Move> {
        let mut moves = Vec::new();

        let player_maps = if gs.current_player == 0 {
            [gs.explorer_p1, gs.vanguard_p1, gs.sentinel_p1]
        } else {
            [gs.explorer_p2, gs.vanguard_p2, gs.sentinel_p2]
        };
        let pieces = [Pieces::Explorer, Pieces::Vanguard, Pieces::Sentinel];

        for i in 0..3 {
            let mut map = player_maps[i];
            while map != 0 {
                let index = map.trailing_zeros() as i16;
                map &= map - 1;

                let (_, mut attack_moves, can_exit) =
                    MoveGenerator::get_piece_moves(gs, index, pieces[i], utils, board);

                if can_exit {
                    moves.push(Move { from: index, to: -1, piece: pieces[i], captured_piece: Pieces::None, captured: false, exit: true });
                }
                while attack_moves != 0 {
                    let to = attack_moves.trailing_zeros() as i16;
                    attack_moves &= attack_moves - 1;
                    moves.push(Move { from: index, to, piece: pieces[i], captured_piece: gs.get_piece_at(to), captured: true, exit: false });
                }
            }
        }
        moves
    }

}









// liste des règles : 

/* ============================================================================
 * TRAVERSAL — RÈGLES COMPLÈTES DU JEU (arène "speed")
 * Source de vérité : SDK officiel traversal-rust-sdk v0.2.0, dossier core/
 * (config.rs, movegen.rs, rules.rs). Numéros R3–R8 = mêmes références que le SDK.
 * ==========================================================================*/

/* ---------------------------------------------------------------------------
 * PLATEAU & TERRAIN
 * - Grille 10×10, cases indexées 0..99, sq = y * 10 + x (y = rangée, x = colonne).
 * - Chaque case a une hauteur 0..6, fixée pour toute la partie (mode de terrain
 *   + graine côté serveur : desert, highlands, mountains, valleys, woodlands).
 * - Terrains générés avec |Δh| <= 2 entre voisins orthogonaux (MAX_STEP).
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * R3 — PIÈCES (6 par camp, mouvement + force = 6)
 *   Explorateur : mouvement 5, force 1
 *   Avant-garde : mouvement 4, force 2
 *   Sentinelle  : mouvement 3, force 3
 *
 * R3.2 — MISE EN PLACE (arrangement "speed", symétrie rotative 180°)
 *   Camp 1 (rouge) : rangée 0, colonnes 2..7 = S V E E V S
 *                    (Sentinel en 2 et 7, Vanguard en 3 et 6,
 *                     Explorer en 4 et 5)
 *   Camp 2 (bleu)  : rangée 9, mêmes pièces en x = 9 - x_rouge.
 *   Camp 1 joue toujours en premier.
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * R4 — DÉPLACEMENT
 *   Coût d'une arête (step_cost) : 1 si hauteur égale, sinon |Δh| + 1.
 *   Budget d'une pièce = son mouvement (ci-dessus), dépensé sur le chemin.
 *   Portée = Dijkstra depuis la case de la pièce, À TRAVERS cases VIDES
 *   seulement, borné par le budget. Les cases occupées (alliées ou ennemies)
 *   bloquent le chemin — jamais de passage au-dessus d'une pièce.
 *   Coup "Move" : atteindre toute case vide de portée (jamais la case de
 *   départ). Le coût du coup = la distance Dijkstra retenue.
 *   R4.3 : sur le plat chaque pas coûte 1 ; monter/descendre coûte |Δh|+1.
 *   R4.8 : si le camp au trait n'a AUCUN coup légal, il joue "Pass" (ce n'est
 *   pas une défaite — pas de règle de zugzwang en arène).
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * R7 — ATTAQUES (mêlée)
 *   Cible : pièce ennemie sur une case ADJACENTE (orthogonale) d'une case vide
 *   de portée. Coût total = dist(case vide) + step_cost(case vide -> case cible),
 *   en prenant le chemin le moins cher, <= budget.
 *
 *   Forces (R7.3/R7.4) :
 *     attaque = force de l'attaquante
 *             + 1 si h(case attaquante) > h(case cible)   (hauteur)
 *             - 1 si h(case attaquante) < h(case cible)
 *             + Σ force des ALLIÉS DE L'ATTAQUANT adjacents à la CIBLE
 *               (l'attaquante elle-même exclue — les renforts se comptent
 *                autour de la cible, pas autour de l'attaquante)
 *     défense = force de la cible
 *             + 1 par ALLIÉ DE LA CIBLE adjacent à la cible
 *               (comptage plat : +1 chacun, quelle que soit sa force)
 *
 *   R7.5 — Charge : si l'attaquante est une Avant-garde et si le coût total
 *   de l'attaque est EXACTEMENT 4, attaque += 1.
 *
 *   Le coup n'est généré que si attaque >= défense (l'égalité capture).
 *   Effet (R7.2) : la cible sort du plateau SANS marquer de points ; la
 *   pièce se déplace sur la case de la cible.
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * R6 — SORTIE (finish)
 *   Rangée de sortie : camp 1 -> rangée 9 ; camp 2 -> rangée 0.
 *   Coup "Exit" : légal si une case de la rangée de sortie est atteignable
 *   avec dist + 1 <= budget (le +1 = le pas final hors du plateau).
 *   Effet : la pièce quitte le plateau ; son camp marque SA FORCE en points
 *   (Explorateur +1, Avant-garde +2, Sentinelle +3).
 *   NOTE : la sortie ne passe QUE par la portée normale (Dijkstra) — les
 *   spéciaux de l'Explorateur ne déclenchent jamais un Exit dans le même coup.
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * R5 — SPÉCIAUX DE L'EXPLORATEUR (seule pièce avec des coups spéciaux)
 *   - Diag : (±1, ±1) — un pas en diagonale, au-dessus des hauteurs.
 *   - Leap : (±2, 0) ou (0, ±2) — bond de deux cases en orthogonal,
 *     R5.2 : la case intermédiaire DOIT être vide (jamais au-dessus d'une pièce).
 *   Les deux ignorent totalement les hauteurs et coûtent 0.
 *   CIBLEMENT IMPORTANT (dédoublonnage) : un spécial n'est généré QUE si sa
 *   case d'arrivée n'est pas déjà atteignable par la portée normale (ou par
 *   un autre coup déjà émis pour cette pièce) — le spécial n'ajoute jamais
 *   un doublon d'une case déjà atteignable.
 *   Arrivée sur case vide -> "Move" ; arrivée sur case ennemie -> "Attack"
 *   (R5 : les spéciaux peuvent attaquer), avec les forces R7.3/7.4 mais
 *   SANS bonus de charge ; légal seulement si attaque >= défense.
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * R8 — FIN DE PARTIE
 *   R8.2 — Spécial Sentinelle : si la PREMIÈRE pièce à sortir (tous camps)
 *   est une Sentinelle, son camp gagne immédiatement (raison "sentinel",
 *   3-0). Dès qu'une pièce quelconque sort, l'option est morte pour toujours.
 *   R8.1 — Sinon, la partie s'arrête dès que 2 pièces au total (tous camps)
 *   sont sorties : le camp au score le plus haut gagne (raison "score") ;
 *   égalité = nulle.
 *   R8.3 — Limite de coups (200 demi-coups, les deux joueurs confondus) :
 *   adjudication. Si un score est non nul : score le plus haut gagne,
 *   égalité non nulle = nulle. Si 0-0 : pour chaque camp, la pièce la plus
 *   avancée selon la clé (rangées restantes, -force) — la plus petite gagne ;
 *   un camp sans pièce sur le plateau perd le départage ; sinon nulle.
 *   Il n'y a NI règle de domination (armée vide), NI règle de répétition.
 * ---------------------------------------------------------------------------*/

/* ---------------------------------------------------------------------------
 * PONT ARÈNE / WIRE (contrat du bac à sable)
 *   - Turn { state, moves, seed, fuel } : le serveur fournit la position,
 *     la liste EXPLICITE des coups légaux (rendez-vous de service : votre
 *     movegen doit coïncider, mais l'arène joue l'indice que vous renvoyez),
 *     une graine par coup, et le fuel. Renvoyer un indice hors liste = défaite.
 *   - Les pièces voyagent en LISTE COMPACTE : la capture d'une pièce décale
 *     tous les indices suivants — reconvertir positions ET indices avec soin.
 *   - Camp 1 = rouge (rangée 0, sort rangée 9) ; camp 2 = bleu (rangée 9,
 *     sort rangée 0). "finished" compte les sorties tous camps confondus.
 *   - 2×10⁹ fuel par coup, remis à neuf chaque coup ; à zéro, partie perdue.
 *   - 64 Mo de mémoire, module <= 1 Mo, pas de threads ; ni fichiers, ni
 *     réseau, ni horloge — pas d'Instant en wasm : budget via fuel_left().
 *   - La mémoire du bot persiste d'un coup à l'autre pendant la partie.
 *   - Un plantage fait perdre la partie. Le hasard ne vient QUE de la graine.
 * ---------------------------------------------------------------------------*/

