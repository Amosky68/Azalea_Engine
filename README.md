

# Plan a suivre : 



### Étape 1 : Les fondations de données (La carte du monde)
- Avant de bouger, le moteur doit comprendre ce qui existe. Restez sur des structures simples (KISS).
- Les Coordonnées : Créez une structure pour représenter une case (ex: x, y ou un index 1D de 0 à 99).
- Les Pièces : Définissez un enum pour le type de pièce (Vanguard, Sentinel, Explorer, etc.) et leur couleur (Rouge/Bleu).
- Le Coup (Move) : Une structure contenant l'origine, la destination, et éventuellement les effets secondaires (capture, modification de hauteur).
- Le Plateau (Board) : La structure centrale contenant la grille (un tableau plat [Option<Piece>; 100]), les hauteurs des cases, et à qui le tour de jouer.


### Étape 2 : Le moteur des règles (Génération de coups)
- C'est le cœur de la conformité. Le but est de créer une fonction generate_legal_moves(&self) -> Vec<Move>.
- Mouvements pseudo-légaux : Codez d'abord la géométrie brute (ex: le Vanguard bouge en diagonale).
- Filtrage physique : Ajoutez les contraintes de collision (ne pas traverser une pièce, respecter les limites du plateau).
- Règles topographiques : Intégrez les règles spécifiques à Traversal (différences de hauteurs maximales autorisées pour monter/descendre).
- Règles spéciales : Codez scrupuleusement la condition de victoire par exfiltration de la Sentinelle (le fameux "Sentinel Rush" qui a posé problème dans la v1).


### Étape 3 : La machine à voyager dans le temps (Make / Undo)
- Pour qu'un bot puisse réfléchir plus tard, il doit pouvoir imaginer le futur et revenir en arrière sans cloner tout le plateau à chaque fois.
- make_move(&mut self, move: Move) : Applique le coup, met à jour le plateau, gère les captures, et passe le tour à l'adversaire.
- undo_move(&mut self, move: Move) : Restaure exactement le plateau dans son état précédent.
- Astuce : Votre structure Move ou l'historique du Board devra mémoriser ce qui a été capturé pour pouvoir le remettre en place lors de l'annulation.


### Étape 4 : Le filet de sécurité (Validation PERFT)
-  C'est l'étape la plus critique. Interdiction de passer à la suite tant que cette étape n'est pas parfaite.
- Implémentez une fonction de comptage récursif des nœuds (PERFT).
- Générez l'arbre des coups depuis la position initiale jusqu'à la profondeur 2, 3 et 4.
- Comparez le nombre exact de feuilles obtenues avec les chiffres du moteur officiel. S'il vous manque 1 seul coup sur 10 000, c'est qu'il y a un bug dans l'Étape 2 ou 3.


### Étape 5 : L'interface de communication (Le traducteur)
- Votre logique de jeu interne doit communiquer avec l'arène web/serveur.
- Parsing JSON : Créez une fonction qui lit le fichier JSON ou la requête de l'arène (contenant les positions, les hauteurs) et le convertit en votre structure Board interne.
- Formatage de sortie : Créez la fonction inverse qui prend votre structure Move et la formate selon le standard attendu par l'API de l'arène (ex: {"from": "c2", "to": "d3"}).


### Étape 6 : Le bot "Zéro IQ" (Random Bot)
- Raccordez le tout pour prouver que la chaîne de bout en bout fonctionne.
- Recevez l'état du serveur.
- Instanciez le Board.
- Appelez generate_legal_moves().
- Choisissez un coup aléatoirement via la caisse rand (moves.choose(&mut rng)).
- Renvoyez le coup au serveur.