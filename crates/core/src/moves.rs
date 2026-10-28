use crate::gamestate::GameState;
use crate::gamestate::Pieces;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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




pub struct MoveGenerator;

impl MoveGenerator {

    // true si une pièce allié attaquer une pièce ennemie
    pub fn can_piece_attack(gs: &GameState, piece: Pieces, from : i16, to : i16, move_cost: i16) -> bool {
    
        let mut piece_allied_force = piece.get_attack_force();

        piece_allied_force += if gs.board.get_height(from) > gs.board.get_height(to) { 1 } else { 0 }; 
        piece_allied_force += if gs.board.get_height(from) < gs.board.get_height(to) { -1 } else { 0 }; 

        for neighbor in gs.board.get_neighbor_indices(to) {
            if gs.is_occupied_by_ally(neighbor, gs.current_player) && from != neighbor {
                piece_allied_force += gs.get_piece_at(neighbor).get_attack_force();
            }
        }

        // charge de l'avant garde : si l'attaquante est une Avant-garde et si le coût total de l'attaque est EXACTEMENT 4, attaque += 1.
        if piece == Pieces::Vanguard && move_cost == 4 {
            piece_allied_force += 1;
        }

        let mut piece_enemy_force = gs.get_piece_at(to).get_attack_force();

        for neighbor in gs.board.get_neighbor_indices(to) {
            if gs.is_occupied_by_enemy(neighbor, gs.current_player) {
                piece_enemy_force += 1;
            }
        }

        piece_allied_force >= piece_enemy_force
    }


    pub fn get_explorer_special_moves(gs: &GameState, index: i16, reached_squares: &std::collections::HashSet<i16>) -> Vec<Move> {
        let mut moves = Vec::new();
       
        // diagonal moves
        for neighbor in gs.board.get_diagnonal_neighbor_indices(index) {
            if !reached_squares.contains(&neighbor) {
                if gs.is_occupied_by_ally(neighbor, gs.current_player) {
                    continue;
                }

                if gs.is_occupied_by_enemy(neighbor, gs.current_player) {
                    if MoveGenerator::can_piece_attack(gs, Pieces::Explorer, index, neighbor, 0) {
                        moves.push(Move {
                            from: index,
                            to: neighbor,
                            piece: Pieces::Explorer,
                            captured_piece: gs.get_piece_at(neighbor),
                            captured: true,
                            exit: false,
                        });
                    }
                    continue;
                }
                moves.push(Move {
                    from: index,
                    to: neighbor,
                    piece: Pieces::Explorer,
                    captured_piece: Pieces::None,
                    captured: false,
                    exit: false,
                });
            }
        }


        // leap moves
        for (neighbor2, between) in gs.board.get_neighbors_two_steps_away(index) {
            if !reached_squares.contains(&neighbor2) && !gs.is_occupied(between) {
                if gs.is_occupied_by_ally(neighbor2, gs.current_player) {
                    continue;
                }

                if gs.is_occupied_by_enemy(neighbor2, gs.current_player) {
                    if MoveGenerator::can_piece_attack(gs, Pieces::Explorer, index, neighbor2, 0) {
                        moves.push(Move {
                            from: index,
                            to: neighbor2,
                            piece: Pieces::Explorer,
                            captured_piece: gs.get_piece_at(neighbor2),
                            captured: true,
                            exit: false,
                        });
                    }
                    continue;
                }

                moves.push(Move {
                    from: index,
                    to: neighbor2,
                    piece: Pieces::Explorer,
                    captured_piece: Pieces::None,
                    captured: false,
                    exit: false,
                });
            }
        }



        moves
    }

    
    /// Returns a list of all squares that can be reached by the given piece.
    pub fn get_piece_moves(gs: &GameState, index: i16, piece: Pieces) -> Vec<Move> {
        let mut moves = Vec::new();
        let movement_budget = piece.get_movement_budget();
        let mut can_exit = false;

        let mut heap = BinaryHeap::new();
        let mut min_cost: HashMap<i16, u32> = HashMap::new();

        min_cost.insert(index, 0);
        heap.push(Reverse((0, index)));


        while let Some(Reverse((current_cost, current_index))) = heap.pop() {
            if current_cost > *min_cost.get(&current_index).unwrap_or(&u32::MAX) {
                continue;
            }

            for neighbor in gs.board.get_neighbor_indices(current_index) {
                // Seules les cases VIDES permettent de se déplacer et de continuer le chemin
                if gs.is_occupied(neighbor) {
                    continue;
                }

                let height_diff = (gs.board.get_height(neighbor) - gs.board.get_height(current_index)).abs() as u32;
                let step_cost = height_diff + 1;
                let new_cost = current_cost + step_cost;

                if new_cost > movement_budget as u32 {
                    continue;
                }

                // Test de sortie du plateau (sur case vide)
                if gs.board.is_neighboring_exit(neighbor, gs.current_player) && (new_cost + 1 <= movement_budget as u32) {
                    can_exit = true;
                }

                // Enregistrement du meilleur coût vers cette case vide
                if new_cost < *min_cost.get(&neighbor).unwrap_or(&u32::MAX) {
                    min_cost.insert(neighbor, new_cost);
                    heap.push(Reverse((new_cost, neighbor)));
                }
            }
        }

        if gs.board.is_neighboring_exit(index, gs.current_player) {
            can_exit = true;
        }
    

        for (&sq, _d) in min_cost.iter() {
            if sq != index { 
                moves.push(Move {
                    from: index,
                    to: sq,
                    piece,
                    captured_piece: Pieces::None,
                    captured: false,
                    exit: false,
                });
            }
        }



        //  Recherche des coûts d'attaque minimaux par cible ennemie
        let mut attack_info: HashMap<i16, (u32, i16)> = HashMap::new();
            for (&sq, &d) in min_cost.iter() {
                for target in gs.board.get_neighbor_indices(sq) {
                    if gs.is_occupied_by_enemy(target, gs.current_player) {
                        let dh = (gs.board.get_height(target) - gs.board.get_height(sq)).abs() as u32;
                        let cost = d + dh + 1;

                        if cost <= movement_budget as u32 {
                            let entry = attack_info.entry(target).or_insert((u32::MAX, sq));
                            if cost < entry.0 {
                                *entry = (cost, sq);
                            }
                        }
                    }
                }
            }

        // coups d'attaque légaux 
        for (target, (cost, _staging)) in attack_info {
            if MoveGenerator::can_piece_attack(gs, piece, index, target, cost as i16) {
                moves.push(Move {
                    from: index,
                    to: target,
                    piece,
                    captured_piece: gs.get_piece_at(target),
                    captured: true,
                    exit: false,
                });
            }
        }

        // Coups spéciaux de l'Explorateur (on transmet les cases visitées sous forme de Vec ou Set)
        if piece == Pieces::Explorer {
            let emitted: std::collections::HashSet<i16> = moves
                .iter()
                .map(|m| m.to)
                .filter(|&t| t >= 0)
                .collect();

            let special_moves = MoveGenerator::get_explorer_special_moves(gs, index, &emitted);
            moves.extend(special_moves);
        }

        if can_exit {
            moves.push(Move {
                from: index,
                to: -1,
                piece,
                captured_piece: Pieces::None,
                captured: false,
                exit: true,
            });
        }

        moves
    }

    
    pub fn generate_all_moves(gs: &GameState) -> Vec<Move> {
        let mut moves = Vec::new();

        for index in 0..100 {           
            if gs.is_occupied_by_ally(index, gs.current_player) {

                let piece_moves = MoveGenerator::get_piece_moves(gs, index, gs.get_piece_at(index));
                moves.extend(piece_moves);
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

