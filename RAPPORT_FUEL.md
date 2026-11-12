# Rapport — où part le fuel, et comment en dépenser 2× moins par nœud

Cible : réduire le fuel par nœud **sans changer un seul coup joué** (logique et arbre de recherche identiques).
Toutes les optimisations proposées ici sont *exactes* : elles produisent le même arbre, donc le même Elo par construction.
Le seul gain d'Elo vient du surplus de nœuds explorés à budget constant.

État de référence : `azalea_arena_1975.wasm` / source actuelle, 1975 Elo.

---

## 0. Résumé

| Action | Gain mesuré | Gain estimé | Risque Elo |
|---|---|---|---|
| **1. Réécrire 4 boucles de `evaluate_from_player_side`** | **−7,8 % de fuel total** (−14,8 % sur l'éval), arbre identique sur 800/800 plies | +5 à 8 % de plus en fusionnant les boucles restantes | **nul** (même arbre) |
| **2. Ordonnancement : bits statiques précalculés + balayage des seuls codes utilisés** | **−11,7 % de fuel total** (160 plies, 622 556 nœuds identiques) | — | **nul** (même arbre vérifié) |
| **Total** | **−18,6 %** (7,8 + 11,7) | | |

Mesures de contrôle déjà faites :
- **Tri par insertion à la place du tri au comptage : +16,9 % de fuel** (même arbre, 264 269 nœuds identiques). → à ne surtout pas faire : les listes de coups font **65,6 coups** en moyenne.
- Le compteur `nodes` **sous-estime le travail** : 769 056 nœuds comptés pour 2 875 366 évaluations (3,7 éval/nœud compté) et 3,58 `apply_move`/nœud. Le ratio « fuel / nœud » n'est donc pas une constante du moteur, il dépend de la position et du budget.

---

## 1. Méthode (pourquoi les chiffres du log ne suffisent pas)

Le fuel compte des **instructions wasm**, pas du temps : caches, taille de la TT, localité mémoire sont hors sujet. Ce qui compte est le nombre d'instructions exécutées.

Pour attribuer le fuel, j'ai construit une copie jetable du crate avec des compteurs de fuel par phase (`fuel_left()` autour de chaque bloc) et un plafond `max_depth = 5` (pour que l'arbre soit identique entre deux builds et que le budget de 2 G ne soit jamais atteint). Duel 4 parties, 800 plies, mêmes graines, puis comparaison **ply par ply**.

Deux builds comparés :
- **A** : instrumenté, avant optimisation.
- **B** : instrumenté + patch d'évaluation.

Résultat de contrôle : **800/800 plies avec un nombre de nœuds strictement identique, 769 056 nœuds au total dans les deux builds** ⇒ même arbre, mêmes coups ⇒ Elo identique.

> Le code du moteur n'a **pas** été modifié : tout a été fait dans une copie de travail supprimée après mesure.

### Le « 32 670 fuel / nœud » est un ratio instable

`stats.txt` (31 522 plies, 572,7 M nœuds) donne bien 32 670 en moyenne, mais :
- le ratio par ply va de 16 000 à 65 000 ;
- avec `--fuel 2G`, le moteur atteint la profondeur 7-13 et le ratio monte à 40 000-65 000, parce que **la dernière itération d'approfondissement, interrompue, brûle du fuel sans produire de nœuds comptés** ;
- une éval sur un nœud « stand-pat » coupé n'est pas comptée dans `nodes`.

Conclusion : pour mesurer une optimisation, il faut comparer **le fuel à arbre identique** (même profondeur, mêmes nœuds), pas le ratio du log.

---

## 2. Répartition mesurée du fuel (build A, 800 plies, 28 329 fuel/nœud)

| Phase | fuel/nœud | part | coût unitaire mesuré |
|---|---:|---:|---|
| **Évaluation statique** | 14 996 | **52,9 %** | 4 011 fuel/appel, **3,7 appels par nœud** |
| **Génération des coups** | 5 214 | **18,4 %** | 22 358 fuel/génération, **65,6 coups générés** |
| **Ordonnancement (codes + tri)** | 5 511 | **19,5 %** | 23 629 fuel par nœud trié |
| `apply_move` | 1 019 | 3,6 % | 284 fuel/appel, 3,58 appels/nœud |
| Hash Zobrist + probe TT | 174 | 0,6 % | |
| Écriture TT | 23 | 0,1 % | |
| Reste (récursion, killer, LMR…) | 1 389 | 4,9 % | |
| **Total** | **28 329** | 100 % | |

Trois faits qui commandent tout le reste :

1. **L'évaluation est la moitié du budget** et elle est appelée **3,7 fois par nœud compté**.
2. **65,6 coups sont générés et triés par nœud, mais 3,58 seulement sont réellement examinés** (coupure alpha-bêta). On paie donc ~95 % des codes d'ordonnancement pour rien.
3. Le reste (hash, TT, apply) est déjà marginal : l'optimiser ne rapporterait presque rien.

---

## 3. Action 1 — évaluation : −7,8 % de fuel total (MESURÉ)

`Evaluator::evaluate` appelle deux fois `evaluate_from_player_side`, et chaque appel fait :
- un parcours de rangées pour la pièce la plus avancée (`get_closest_piece_to_goal`),
- un parcours des **10 rangées** pour la somme des distances,
- un parcours de rangées pour la sentinelle (`sentinel_distance_to_exit`),
- une boucle sur **toutes** les pièces alliées pour compter les escortes (avec `1u128 << sq` et classification à chaque tour).

Les quatre remplacements ci-dessous donnent **exactement les mêmes valeurs** (mêmes entiers, mêmes cas limites) en supprimant les boucles.

### 3.1 Pièce la plus avancée (au lieu du parcours de rangées)

```rust
// Remplace l'appel à Evaluator::get_closest_piece_to_goal
if ally_map != 0 {
    let sq: i32 = if player == 0 {
        127 - ally_map.leading_zeros() as i32      // rangée la plus haute
    } else {
        ally_map.trailing_zeros() as i32           // rangée la plus basse
    };
    let row = sq / 10;
    let in_row = ally_map & utils.get_row_mask(row as i16);
    // même règle que l'original : la pièce la PLUS FORTE de cette rangée
    let closest_piece = if ally_sentinel & in_row != 0 {
        Pieces::Sentinel
    } else if ally_vanguard & in_row != 0 {
        Pieces::Vanguard
    } else {
        Pieces::Explorer
    };
    let distance = if player == 0 { 9 - row } else { row };
    score += (10 - distance) * closest_piece.get_attack_force() as i32
        * weights.closest_piece_multiplier;
}
```

### 3.2 Somme des distances : boucle sur les pièces, pas sur les rangées

```rust
// Remplace le `for row in 0..10 { ... count_ones ... }`
let mut dist_map = ally_map;
while dist_map != 0 {
    let sq = dist_map.trailing_zeros() as i32;
    dist_map &= dist_map - 1;
    let rows_to_go = if player == 0 { 9 - sq / 10 } else { sq / 10 };
    score += (10 - rows_to_go) * weights.distance_multiplier;
}
```

### 3.3 Distance de la sentinelle (au lieu du parcours de rangées)

```rust
// Remplace l'appel à Evaluator::sentinel_distance_to_exit
let sentinel_distance = {
    let map = if player == 0 { state.sentinel_p1 } else { state.sentinel_p2 };
    if map == 0 {
        10
    } else if player == 0 {
        9 - (127 - map.leading_zeros() as i32) / 10
    } else {
        map.trailing_zeros() as i32 / 10
    }
};
score += (10 - sentinel_distance) * weights.sentinel_distance_multiplier;
```

### 3.4 Escortes : itérer les 3 masques (budget constant par masque)

```rust
// Remplace la boucle `while temp != 0 { let mask = 1u128 << sq; let b = ...; }`
let mut escorts = 0;
let leader_bit = 1u128 << leader_sq;
let mut m = ally_sentinel & !leader_bit;
while m != 0 {
    let sq = m.trailing_zeros() as usize; m &= m - 1;
    if utils.get_free_movement_layout(3, sq) & s_neighbors != 0 { escorts += 1; }
}
let mut m = ally_vanguard;                       // les 3 masques sont disjoints
while m != 0 {
    let sq = m.trailing_zeros() as usize; m &= m - 1;
    if utils.get_free_movement_layout(4, sq) & s_neighbors != 0 { escorts += 1; }
}
let mut m = ally_explorer;
while m != 0 {
    let sq = m.trailing_zeros() as usize; m &= m - 1;
    if utils.get_free_movement_layout(5, sq) & s_neighbors != 0 { escorts += 1; }
}
score += escorts.min(2) * weights.sentinel_escort_bonus;
```

### 3.5 Résultat mesuré

| | avant | après | Δ |
|---|---:|---:|---:|
| fuel / appel d'éval | 4 011 | **3 417** | **−14,8 %** |
| part de l'éval | 52,9 % | 48,9 % | −4 pts |
| fuel / nœud | 28 329 | **26 111** | **−7,8 %** |
| nœuds (800 plies) | 769 056 | 769 056 | **0** |

Les deux fonctions `get_closest_piece_to_goal` et `sentinel_distance_to_exit` deviennent inutilisées (à supprimer ou à garder pour le tuner).

### 3.6 Pour aller plus loin sur l'éval

Après ce patch, l'éval reste ~49 % du total. Les blocs dominants sont la boucle d'escorte + menace (≈ 40 % de l'éval) et le calcul des masques d'entrée. Deux pistes exactes, à mesurer :
- **fusionner** la boucle « somme des distances » (§3.2) et la boucle d'escorte en une seule passe sur `ally_map` (un seul `trailing_zeros` par pièce) ;
- **porter les termes de distance dans une évaluation incrémentale** (delta dans `apply_move`) — c'est le seul chemin vers un facteur 2 sur l'éval, mais c'est un refactor lourd et il faut être bit-exact.

---

## 4. Action 2 — ordonnancement : −11,7 % de fuel total (MESURÉ)

Mesures : **65,6 coups générés/triés par nœud, 3,58 examinés**, 23 629 fuel par nœud trié, soit ~360 fuel par coup. Le coût se décompose en :
- `Evaluator::move_ordering_code` appelé **pour chaque coup** (~330 fuel/coup) : 2 comparaisons tt, 4 comparaisons killer, 3 tests de drapeaux, un `match` sur la pièce, **deux `Board::row_of`** (accès table + test de borne) ;
- le tri au comptage : ~1 800 fuel, dont ~1 300 pour le **balayage des 256 compteurs** (et 1 Ko de tableaux remis à zéro) à chaque nœud, quelle que soit la taille de la liste.

### 4.1 Précalculer la partie statique du code à la génération

Les bits `exit`, `captured`, `sentinel`, `forward` et `force` ne dépendent que du coup et du camp au trait (constant pendant la génération). Les calculer une fois, dans un buffer parallèle `[u8; 256]` rempli par `MoveGenerator::generate_all_moves` / `generate_forcing_moves`, et ne laisser dans `move_ordering_code` que :

```rust
// code = bits statiques précalculés | tt | killer
let mut code = self.move_codes[depth][i];
if mv.from == tt_move.from && mv.to == tt_move.to { code |= 1 << 7; }
if (mv.from == killer_1.from && mv.to == killer_1.to)
    || (mv.from == killer_2.from && mv.to == killer_2.to) { code |= 1 << 6; }
```

À la génération, `row_of(from)` est calculé **une fois par pièce** (pas par coup) et `force`/`sentinel` sont des constantes de la boucle de pièce : le coût passe d'environ 70-80 instructions à 15-20 par coup. Gain mesuré (avec 4.2) : **−11,72 % de fuel total**, soit 23 629 → 20 859 fuel/nœud, avec **622 556 nœuds identiques et 160/160 plies identiques** (build plafonné à `max_depth = 5`, même adversaire, mêmes graines).

⚠️ L'ordre produit doit rester **exactement** celui du tri au comptage : décroissant sur `code`, et à code égal, ordre de génération croissant.

### 4.2 Ne balayer que les codes réellement présents

Dans `counting_sort`, remplacer le balayage `for c in (0..256).rev()` par un balayage des seuls codes utilisés (bitset `[u64; 4]`, ~15 codes distincts en pratique) :

```rust
let mut used = [0u64; 4];
for &c in codes { used[(c >> 6) as usize] |= 1u64 << (c & 63); }
// ... comptage ...
let mut pos = [0u16; 256];
let mut acc = 0u16;
for w in (0..4).rev() {
    let mut bits = used[w];
    while bits != 0 {
        let b = 63 - bits.leading_zeros() as usize;
        bits &= !(1u64 << b);
        let c = (w << 6) + b;
        pos[c] = acc;
        acc += counts[c];          // exactement le même ordre qu'avant
    }
}
```

Gain : ~1 100 fuel par tri (≈ −1 % du total). Combiné à 4.1 : **−11,72 % mesurés**.

### 4.3 Ce qu'il ne faut pas faire

- **Tri par insertion / par comparaison** : mesuré **+16,9 % de fuel total** à arbre identique (264 269 nœuds). Avec 65 coups par nœud, l'O(n²) coûte plus cher que les 256 seaux.
- Toute « optimisation » de l'ordre des coups (bucket plus grossier, tri partiel) change l'arbre ⇒ risque d'Elo. À réserver à un SPRT dédié.

---

## 5. Petits postes sûrs (facultatifs)

| Poste | Gain | Commentaire |
|---|---|---|
| Hash Zobrist incrémental | ~0,6 % | `apply_move` met à jour la clé (xor des clés de case, de score et de sortie) au lieu de recalculer `get_hash` sur les 12 pièces à chaque nœud. Attention à rester bit-exact. |
| `can_piece_attack` | ~1-2 % de la génération | Les masques allié/ennemi et `neighbor_mask(to)` sont recalculés **par cible** ; les hoister au niveau de `get_piece_moves`. |
| Sortie immédiate si `self.stopped` dans les boucles de coups | marginal | Aujourd'hui, quand le budget est épuisé, chaque coup restant paie encore `apply_move` + une évaluation complète. |

---

## 6. Protocole de validation (à appliquer à chaque patch)

1. **Preuve d'équivalence** : plafonner `max_depth` (4 ou 5), `--fuel 2000000000` (pour ne jamais couper sur le fuel), 4 parties, comparer **le nombre de nœuds ply par ply** avant/après. Identique partout ⇒ arbre identique ⇒ Elo identique par construction.
2. **Mesure du gain** : à arbre identique, comparer le fuel par ply et le fuel/nœud (le seul chiffre qui compte ici).
3. **Puis seulement** : SPRT 400 parties contre `azalea_arena_1975.wasm` (`scripts/sprt.sh`) — le gain d'Elo vient du surplus de nœuds/depth à budget égal, pas du patch lui-même.
4. Ne jamais comparer des « fuel/nœud » issus d'arbres différents (cf. §1).

---

## 7. Réserves

- Les parts relatives (éval ≈ 53 %, génération ≈ 18 %, tri ≈ 20 %) sont stables ; les valeurs absolues dépendent de la phase et du budget de fuel.
- L'action 1 est **mesurée** (arbre identique). L'action 2 est une estimation à confirmer avec le protocole du §6 avant adoption.
- Le harness local s'arrête à ~602 M de fuel par coup (budget par défaut) ; sur l'arène réelle (2 G), le moteur va plus profond et la répartition penche encore plus vers l'éval (3,7 évaluations par nœud compté).
