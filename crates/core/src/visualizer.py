import json
import numpy as np
import matplotlib.pyplot as plt

# ==========================================
# 1. CHARGEMENT DES CARTES
# ==========================================
MAPS_FILE = "synergy_maps.json"

try:
    with open(MAPS_FILE, 'r', encoding='utf-8') as f:
        synergy_maps = json.load(f)
    print(f"Cartes chargées avec succès depuis {MAPS_FILE}")
except FileNotFoundError:
    print(f"Erreur : Le fichier {MAPS_FILE} est introuvable. Exécutez d'abord precalc.py.")
    exit()

# Choix de la paire à visualiser : 'sv', 'se', 've', 'ss', 'vv', 'ee'
# Exemple: 'sv' = Pièce 1 est une Sentinelle, Pièce 2 est une Avant-garde
PAIRE_ACTUELLE = 'se'

# Noms lisibles pour l'affichage
PIECE_NAMES = {
    's': 'Sentinelle',
    'v': 'Avant-garde',
    'e': 'Explorateur'
}

p1_type = PIECE_NAMES[PAIRE_ACTUELLE[0]]
p2_type = PIECE_NAMES[PAIRE_ACTUELLE[1]]

# Position initiale de la pièce 1 (par exemple au centre : x=4, y=4 -> sq=44)
p1_sq = 44

# ==========================================
# 2. CONFIGURATION DU GRAPHIQUE INTERACTIF
# ==========================================
fig, ax = plt.subplots(figsize=(8, 8))
plt.subplots_adjust(bottom=0.15)

def get_grid_data(pair, sq1):
    """
    Extrait les 100 valeurs de synergie pour la pièce 2 
    lorsque la pièce 1 est fixe en sq1, et les met sous forme de grille 10x10.
    """
    matrix = np.array(synergy_maps[pair])  # Forme (100, 100)
    scores = matrix[sq1, :]                # Vecteur de 100 valeurs pour la pièce 2
    grid = scores.reshape((10, 10))        # Grille 10x10 (y, x)
    return grid

# Affichage initial de la heatmap
grid = get_grid_data(PAIRE_ACTUELLE, p1_sq)
im = ax.imshow(grid, cmap='RdYlGn', vmin=-1.0, vmax=1.0, origin='lower')
cbar = fig.colorbar(im, ax=ax, label="Probabilité / Impact de Victoire (-1 à +1)")

# Marquer la position de la pièce 1 fixe avec une étoile
p1_x, p1_y = p1_sq % 10, p1_sq // 10
marker, = ax.plot(p1_x, p1_y, marker='*', color='blue', markersize=18, markeredgecolor='black', label=f"{p1_type} (Fixe)")

# Mise en forme du plateau
ax.set_xticks(range(10))
ax.set_yticks(range(10))
ax.grid(True, color='black', linewidth=0.5)
ax.set_title(f"Synergie : Cliquez pour déplacer {p1_type} (★)\nLa heatmap montre le score de {p2_type}")
ax.legend(loc='upper right')

# ==========================================
# 3. GESTION DU CLIC SOURIS
# ==========================================
def on_click(event):
    global p1_sq
    
    # Vérifie si le clic est bien à l'intérieur du plateau
    if event.inaxes == ax and event.xdata is not None and event.ydata is not None:
        x = int(round(event.xdata))
        y = int(round(event.ydata))
        
        if 0 <= x < 10 and 0 <= y < 10:
            p1_sq = y * 10 + x
            
            # Mettre à jour la grille de valeurs
            new_grid = get_grid_data(PAIRE_ACTUELLE, p1_sq)
            im.set_data(new_grid)
            
            # Mettre à jour la position de l'étoile
            marker.set_data([x], [y])
            
            # Mettre à jour le titre avec le score exact sur la case sous la souris
            ax.set_title(f"{p1_type} placée en ({x}, {y}) [Case {p1_sq}]\nHeatmap = Répartition préférentielle pour {p2_type}")
            fig.canvas.draw_idle()

# Connecter l'événement de clic
fig.canvas.mpl_connect('button_press_event', on_click)

print(f"Visualisateur démarré pour la paire '{PAIRE_ACTUELLE}' ({p1_type} / {p2_type}).")
print("Cliquez n'importe où sur la grille pour déplacer la pièce fixe !")

plt.show()