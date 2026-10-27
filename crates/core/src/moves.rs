use crate::gamestate::GameState;
use crate::gamestate::Pieces;



pub struct Move {
    pub from: i16, // indice de la case de départ 
    pub to: i16,   // indice de la case d'arrivée
    pub piece: Pieces, // pièce qui est déplacée

    //pub piece: Piece, // pièce qui est déplacée

    pub captured: bool,
    pub exit: bool, // true si le mouvement est une sortie de pièce du plateau 
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




impl MoveGenerator {

    // true si une pièce allié attaquer une pièce ennemie
    pub fn can_piece_attack(&self, gs: &GameState, piece: Pieces, from : i16, to : i16, move_cost: i16) -> bool {
    
        let piece_allied_force = piece.get_attack_force();

        piece_allied_force += if gs.board.get_height(from) > gs.board.get_height(to) { 1 } else { 0 }; 
        piece_allied_force += if gs.board.get_height(from) < gs.board.get_height(to) { -1 } else { 0 }; 

        for neighbor in gs.board.get_neighbor_indices(to) {
            if gs.is_occupied_by_ally(neighbor, gs.current_player) {
                piece_allied_force += gs.get_piece_at(neighbor).get_attack_force();
            }
        }

        // charge de l'avant garde : si l'attaquante est une Avant-garde et si le coût total de l'attaque est EXACTEMENT 4, attaque += 1.
        if piece == Pieces::AvantGarde && move_cost == 4 {
            piece_allied_force += 1;
        }

        let piece_enemy_force = gs.get_piece_at(to).get_attack_force();

        for neighbor in gs.board.get_neighbor_indices(to) {
            if gs.is_occupied_by_enemy(neighbor, gs.current_player) {
                piece_enemy_force += 1;
            }
        }

        piece_allied_force >= piece_enemy_force
    }



    pub fn get_explorer_special_moves(&self, gs: &GameState, index: i16, reached_squares: HashSet<i16>) -> Vec<Move> {
        let mut moves = Vec::new();
       
        // diagonal moves
        for neighbor in gs.board.get_diagnonal_neighbor_indices(index) {
            if !reached_squares.contains(&neighbor) {
                if (gs.is_occupied_by_ally(index, player)) {
                    continue;
                }

                if (gs.is_occupied_by_enemy(neighbor, gs.current_player)) {
                    if (self.can_piece_attack(gs, Pieces::Explorer, index, neighbor, 0)) {
                        moves.push(Move {
                            from: index,
                            to: neighbor,
                            piece: Pieces::Explorer,
                            captured: true,
                            exit: false,
                        });
                    }
                } 
            }
        }


        // leap moves
        for (neighbor2, between) in gs.board.get_neighbors_two_steps_away(index) {
            if !reached_squares.contains(&neighbor2) && !reached_squares.contains(&between) {
                if (gs.is_occupied_by_ally(index, player)) {
                    continue;
                }

                if (gs.is_occupied_by_enemy(neighbor2, gs.current_player)) {
                    if (self.can_piece_attack(gs, Pieces::Explorer, index, neighbor2, 0)) {
                        moves.push(Move {
                            from: index,
                            to: neighbor2,
                            piece: Pieces::Explorer,
                            captured: true,
                            exit: false,
                        });
                    }
                } 
            }
        }



        moves
    }

    
    /// Returns a list of all squares that can be reached by the given piece.
    pub fn get_piece_moves(&self, gs: &GameState, index: i16, piece: Pieces) -> Vec<Move> {
        let mut moves = Vec::new();
        let movement_budget = self.get_movement_budget(piece);
        let mut can_exit = gs.board.is_neighboring_exit(index, gs.current_player);

        let mut attacking_move = false;
        let mut add_move = false;


        let queue = VecDeque::new();
        let seen  = HashSet::new();
        queue.push_back((index, 0));

        while let Some((current_index, current_cost)) = queue.pop_front() {
            if current_cost > movement_budget {
                continue;
            }

            for neighbor in gs.board.get_neighbor_indices(current_index) {
                // Coût d'une arête (step_cost) : 1 si hauteur égale, sinon |Δh| + 1.
                let step_cost = abs(gs.board.get_height(neighbor) - gs.board.get_height(current_index)) + 1;
                let new_cost = current_cost + step_cost;
                attackingMove = false;
                addMove = false;


                if (gs.is_occupied_by_ally(neighbor, gs.current_player) || seen.contains(&neighbor) || new_cost > movement_budget) {
                    continue;
                }

                if (gs.board.is_neighboring_exit(neighbor, gs.current_player)) {
                    if (new_cost + 1 <= movement_budget) {
                        addMove = true;
                        canExit = true;
                    }
                }


                // on vérifie l'attaque pour une case ennemie
                if (gs.is_occupied_by_enemy(neighbor, gs.current_player)) {
                    if (self.can_piece_attack(gs, piece, current_index, neighbor, new_cost)) {
                        addMove = true;
                        attackingMove = true;
                    }
                    continue;
                }



                if new_cost <= movement_budget && !gs.is_occupied_by_ally(neighbor, gs.current_player) && !seen.contains(&neighbor) {
                    addMove = true;
                }


                if addMove  {
                    seen.insert(neighbor);
                    queue.push_back((neighbor, new_cost));
                    moves.push(Move {
                        from: index,
                        to: neighbor,
                        piece: piece,
                        captured: false,
                        exit: false,
                    });
                }


            }
        }

        if (piece == Pieces::Explorer) {
            let special_moves = self.get_explorer_special_moves(gs, index, seen);
            moves.extend(special_moves);
        }


        moves
    }

    


    pub fn generate_all_moves(&self, gs: &GameState, index: i16, piece: Pieces) -> Vec<Move> {
        let mut moves = Vec::new();

        for index in 0..100 {           
            if (gs.is_occupied_by_ally(index, gs.current_player)) {

                let piece_moves = self.get_piece_moves(gs, index, piece);
                moves.extend(piece_moves);
            }
        }
        moves
    }




}