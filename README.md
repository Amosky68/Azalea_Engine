

# Azalea - Traversal Engine 


## Description 

Azalea est un moteur du jeu Traversal principalement designé pour la Traversal-arena (https://traversal-bot.app/arena).
Azalea est entièrement écrit en Rust et conçu pour être compilé en WebAssembly (Wasm32) nécéssaire pour l'arena. 


## Architecture 

Le moteur est construit autour d'une architecture standard d'échecs/jeux de plateau déterministes, adaptée aux règles asymétriques et de topologie de Traversal.
Plusieurs technique d'optimisation peuvent être retrouvées ici : 
* Représentation (Bitboards) : Le plateau de 10x10 est représenté via des entiers u128, des mask etc..
* Génération de coups (moves.rs) : Génération pseudo-légale filtrée, séparant les coups calmes (free moves), les attaques et les sorties (exits)
* Minimax avec Alpha-Beta prunning 
* Iterative deepening (on augmente progressivement la depth jusqu'a atteindre la limite de fuel)
* Quiescence Search (profondeur max : 4) ciblant uniquement les captures et sorties pour éviter l'effet d'horizon
* Transposition Table (Zobrist Hashing, ~16 Mo) pour mémoriser les évaluations exactes et les bornes (Upper/Lower)
* Move Ordering. On trie les coups par ordre d'importance (pour rechercher les meilleurs en premier)
* Late Move Reductions (LMR) qui réduit la profondeur de recherche pour les coups moins intéréssant 


L'évaluation statique (d'un gamestate) est ajusté par Texel Tunning et ne comporte que 19 paramètres (voir eval.rs). Les paramètres incluent le différentiel matériel, l'avancée de la course (sentinelles), le contrôle spatial (influence de zone), la couverture défensive (escortes) et la structure topologique du plateau.

