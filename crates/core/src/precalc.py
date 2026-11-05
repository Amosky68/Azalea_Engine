import json
import os
import glob



# Constantes du plateau
NUM_SQUARES = 100
BOARD_SIZE = 10

# Fenêtre de convolution : (dx, dy, poids)
# Correspond à la matrice 3x3 de votre brouillon
WINDOW = [
    (-1, -1, 0.25), (0, -1, 0.50), (1, -1, 0.25),
    (-1,  0, 0.50), (0,  0, 1.00), (1,  0, 0.50),
    (-1,  1, 0.25), (0,  1, 0.50), (1,  1, 0.25)
]

# Les 6 paires de synergies possibles
PAIRS = ['sv', 'se', 've', 'ss', 'vv', 'ee']

# Initialisation des matrices (100x100) pour chaque paire
maps = {p: [[0.0 for _ in range(NUM_SQUARES)] for _ in range(NUM_SQUARES)] for p in PAIRS}
weights = {p: [[5.0 for _ in range(NUM_SQUARES)] for _ in range(NUM_SQUARES)] for p in PAIRS}


def update_pair_map(pair_name, list_pieces_1, list_pieces_2, win_factor):
    """
    Met à jour la matrice lissée pour une paire de pièces donnée.
    """
    m = maps[pair_name]
    w = weights[pair_name]
    
    for sq1 in list_pieces_1:
        for sq2 in list_pieces_2:
            # On empêche de comparer une pièce avec elle-même 
            # (Dans Traversal, 2 pièces ne peuvent pas être sur la même case)
            if sq1 == sq2:
                continue
                
            x1, y1 = sq1 % BOARD_SIZE, sq1 // BOARD_SIZE
            x2, y2 = sq2 % BOARD_SIZE, sq2 // BOARD_SIZE
            
            # Application du smoothing sur la pièce 1
            for dx1, dy1, w1 in WINDOW:
                nx1, ny1 = x1 + dx1, y1 + dy1
                # Vérifie si le décalage reste dans le plateau
                if not (0 <= nx1 < BOARD_SIZE and 0 <= ny1 < BOARD_SIZE): 
                    continue
                nsq1 = ny1 * BOARD_SIZE + nx1
                
                # Application du smoothing sur la pièce 2
                for dx2, dy2, w2 in WINDOW:
                    nx2, ny2 = x2 + dx2, y2 + dy2
                    if not (0 <= nx2 < BOARD_SIZE and 0 <= ny2 < BOARD_SIZE): 
                        continue
                    nsq2 = ny2 * BOARD_SIZE + nx2
                    
                    alpha = w1 * w2
                    
                    # Mise à jour de la moyenne pondérée glissante
                    old_w = w[nsq1][nsq2]
                    m[nsq1][nsq2] = (m[nsq1][nsq2] * old_w + alpha * win_factor) / (old_w + alpha)
                    w[nsq1][nsq2] += alpha


def add_state_to_maps(state):
    """
    Extrait les pièces du dictionnaire JSON et alimente les matrices d'apprentissage.
    """
    # Le label JSON: 1.0 (Victoire Rouge), 0.0 (Victoire Bleu), 0.5 (Nul)
    # Conversion en facteur: 1.0, -1.0, ou 0.0
    win_factor_red = (state['label'] - 0.5) * 2.0
    win_factor_blue = -win_factor_red 
    
    # === JOUEUR 1 (Rouge) ===
    red_e = state['red'].get('e', [])
    red_v = state['red'].get('v', [])
    red_s = state['red'].get('s', [])
    
    # Synergies inter-pièces
    update_pair_map('sv', red_s, red_v, win_factor_red)
    update_pair_map('se', red_s, red_e, win_factor_red)
    update_pair_map('ve', red_v, red_e, win_factor_red)
    # Synergies intra-pièces (entre 2 sentinelles, 2 vanguards...)
    update_pair_map('ss', red_s, red_s, win_factor_red)
    update_pair_map('vv', red_v, red_v, win_factor_red)
    update_pair_map('ee', red_e, red_e, win_factor_red)
    
    
    # === JOUEUR 2 (Bleu) ===
    # On inverse le plateau (99 - sq) pour que la matrice apprenne des 
    # schémas directionnels valables de n'importe quel côté du plateau !
    blue_e = [99 - sq for sq in state['blue'].get('e', [])]
    blue_v = [99 - sq for sq in state['blue'].get('v', [])]
    blue_s = [99 - sq for sq in state['blue'].get('s', [])]
    
    update_pair_map('sv', blue_s, blue_v, win_factor_blue)
    update_pair_map('se', blue_s, blue_e, win_factor_blue)
    update_pair_map('ve', blue_v, blue_e, win_factor_blue)
    update_pair_map('ss', blue_s, blue_s, win_factor_blue)
    update_pair_map('vv', blue_v, blue_v, win_factor_blue)
    update_pair_map('ee', blue_e, blue_e, win_factor_blue)


def load_librairie():
    """
    Parcourt les fichiers JSON du dataset, rejoue les parties et génère 
    la liste des états de jeu (positions) pour l'apprentissage.
    """
    dataset_dir = r"C:\Users\amoro\Documents\Amaury\Programmation\Rust\Traversal\Plays_dataset\banque2200"
    
    # Chercher tous les fichiers JSON dans le dossier
    file_patterns = os.path.join(dataset_dir, "*.json")
    json_files = glob.glob(file_patterns)
    
    librairie = []
    
    print(f"Trouvé {len(json_files)} fichiers JSON. Chargement en cours...")
    
    for file_path in json_files:
        with open(file_path, 'r', encoding='utf-8') as f:
            data = json.load(f)
            
            # Un fichier peut contenir plusieurs parties dans "games"
            for game in data.get("games", []):
                
                # 1. Déterminer le résultat (label)
                winner = game.get("winner")
                if winner == 1:
                    label = 1.0   # Victoire Rouge
                elif winner == 2:
                    label = 0.0   # Victoire Bleu
                else:
                    label = 0.5   # Nul
                
                # 2. Initialiser les pièces (Configuration "speed")
                # Les clés sont les pieceId ("rs1", "be2", etc.), les valeurs sont les indices (0-99)
                pieces_pos = {
                    "rs1": 2, "rv1": 3, "re1": 4, "re2": 5, "rv2": 6, "rs2": 7,  # Rouge (Camp 1)
                    "bs2": 92, "bv2": 93, "be2": 94, "be1": 95, "bv1": 96, "bs1": 97 # Bleu (Camp 2)
                }
                
                # 3. Rejouer les coups
                for move in game.get("moves", []):
                    kind = move.get("kind")
                    piece_id = move.get("pieceId")
                    to_sq = move.get("to")
                    
                    if kind == "move":
                        pieces_pos[piece_id] = to_sq
                        
                    elif kind == "attack":
                        pieces_pos[piece_id] = to_sq
                        captured_id = move.get("capturedId")
                        if captured_id in pieces_pos:
                            del pieces_pos[captured_id]
                            
                    elif kind == "exit":
                        if piece_id in pieces_pos:
                            del pieces_pos[piece_id]
                    
                    # 4. Construire l'état actuel et l'ajouter à la librairie
                    state = {
                        "label": label,
                        "red": {"e": [], "v": [], "s": []},
                        "blue": {"e": [], "v": [], "s": []}
                    }
                    
                    for pid, sq in pieces_pos.items():
                        # pid[0] donne le camp ('r' ou 'b'), pid[1] donne le type ('e', 'v', 's')
                        color = "red" if pid.startswith('r') else "blue"
                        ptype = pid[1] 
                        state[color][ptype].append(sq)
                    
                    librairie.append(state)
                    
    return librairie

def export_maps():
    """
    Exporte les matrices dans un fichier JSON pour pouvoir être lues par Rust.
    """
    def round_map(matrix, decimals=3):
        return [[round(float(val), decimals) for val in row] for row in matrix]

    maps_rounded = {
        pair: round_map(m, 3) 
        for pair, m in maps.items()
    }

    output_path = "synergy_maps.json"
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(maps_rounded, f, separators=(',', ':'))
    print(f"Matrices exportées avec succès dans {output_path} !")


def symetrize_maps():
    """
    Symétrise les matrices pour que score(sq1, sq2) = score(sq2, sq1)
    Uniquement pour les paires de pièces identiques (ss, vv, ee)
    """
    for pair in ['ss', 'vv', 'ee']: # Pas besoin de le faire pour sv, se, ve !
        m = maps[pair]
        for sq1 in range(NUM_SQUARES): 
            for sq2 in range(sq1 + 1, NUM_SQUARES):
                avg = (m[sq1][sq2] + m[sq2][sq1]) / 2.0
                m[sq1][sq2] = avg
                m[sq2][sq1] = avg




def calculate_maps():
    librairie = load_librairie()
    print(f"Lancement de l'apprentissage sur {len(librairie)} positions...")
    
    for i, state in enumerate(librairie):
        add_state_to_maps(state)
        if i > 0 and i % 5000 == 0:
            print(f"{i} positions traitées...")

    print("Symétrisation des matrices...")
    symetrize_maps()
            
    print("Apprentissage terminé. Export des matrices...")
    export_maps()

if __name__ == "__main__":
    calculate_maps()