//! Étape 1 du pipeline Texel : replayer + validation de la banque de parties.
//!
//! Rejoue chaque partie depuis le JSON de l'arène, valide la cohérence
//! structurelle (camps, plis, pièces, captures, sorties, scores, vainqueur),
//! et échantillonne les positions pour l'étape 2 (extraction de features).
//!
//! Autonome par design : ne dépend d'aucune API de azalea_core.

#![allow(dead_code)]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

use serde::Deserialize;

// ============================ Schéma JSON ============================

#[derive(Deserialize, Debug)]
struct BankFile {
    #[serde(default)]
    games: Vec<Game>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Game {
    id: String,
    #[serde(default)]
    terrain: Terrain,
    #[serde(default)]
    setup: String,
    #[serde(default = "default_move_limit")]
    move_limit: u16,
    #[serde(default)]
    winner: Option<u8>, // 1 = rouge, 2 = bleu, None = nulle (absolu, pas relatif au bot)
    #[serde(default)]
    reason: String, // "sentinel" | "score" | "move_limit"
    #[serde(default)]
    ply: u16,
    #[serde(default)]
    scores: [i32; 2], // [rouge, bleu]
    #[serde(default)]
    moves: Vec<MoveRecord>,
}

fn default_move_limit() -> u16 {
    200
}

#[derive(Deserialize, Debug, Default)]
struct Terrain {
    #[serde(default)]
    mode: String,
    #[serde(default)]
    seed: i64,
    #[serde(default)]
    heights: Vec<i16>, // index = row * 10 + col, row 0 = côté rouge
}

#[derive(Deserialize, Debug)]
struct PlayerRef {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct MoveRecord {
    #[serde(default)]
    ply: u16,
    #[serde(default)]
    side: u8, // 1 = rouge, 2 = bleu
    #[serde(default)]
    piece_id: String, // "rs1", "be2", ...
    #[serde(default)]
    kind: String, // "move" | "attack" | "exit"
    #[serde(default)]
    special: Option<String>,
    #[serde(default)]
    from: Option<i16>,
    #[serde(default)]
    to: Option<i16>, // None pour les sorties
    #[serde(default)]
    path: Option<Vec<i16>>,
    #[serde(default)]
    captured_id: Option<String>,
}

fn default_from() -> i16 {
    -1
}

// ============================ Modèle de replay ============================

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PieceKind {
    Explorer,
    Vanguard,
    Sentinel,
}

impl PieceKind {
    /// Points rapportés à la sortie (confirmé par les scores d'arène).
    fn value(self) -> i32 {
        match self {
            PieceKind::Explorer => 1,
            PieceKind::Vanguard => 2,
            PieceKind::Sentinel => 3,
        }
    }
    /// Budget de déplacement — SOFT : à confronter au movegen si warnings.
    fn budget(self) -> i32 {
        match self {
            PieceKind::Explorer => 5,
            PieceKind::Vanguard => 4,
            PieceKind::Sentinel => 3,
        }
    }
}

const PIECE_IDS: [&str; 12] = ["re1", "re2", "rv1", "rv2", "rs1", "rs2", "be1", "be2", "bv1", "bv2", "bs1", "bs2"];

/// Placement initial observé dans les exports ("setup: speed").
/// Si un setup différent existe, l'assert `from` du pli 1 fera feu → à étendre.
const INITIAL_SQUARES: [i16; 12] = [4, 5, 3, 6, 2, 7, 95, 94, 96, 93, 97, 92];

fn piece_info(id: &str) -> (u8, PieceKind) {
    let b = id.as_bytes();
    assert_eq!(b.len(), 3, "pieceId inattendu : {}", id);
    let side = match b[0] {
        b'r' => 1u8,
        b'b' => 2u8,
        _ => panic!("camp inconnu dans pieceId : {}", id),
    };
    let kind = match b[1] {
        b'e' => PieceKind::Explorer,
        b'v' => PieceKind::Vanguard,
        b's' => PieceKind::Sentinel,
        _ => panic!("type inconnu dans pieceId : {}", id),
    };
    (side, kind)
}

fn is_adjacent(a: i16, b: i16) -> bool {
    let d = (a - b).abs();
    d == 10 || (d == 1 && a / 10 == b / 10)
}

struct Piece {
    side: u8,
    kind: PieceKind,
    square: Option<i16>, // None = capturée ou sortie
}

struct ReplayState {
    pieces: Vec<Piece>,
    index_of: HashMap<String, usize>,
    occupied: [Option<usize>; 100],
    scores: [i32; 2],
    exited_piece_count: u32,
    first_exit: Option<(u8, PieceKind)>,
    ply: u16,
    current_side: u8,
    warnings: u32,
}

impl ReplayState {
    fn new() -> Self {
        let mut pieces = Vec::with_capacity(12);
        let mut index_of = HashMap::new();
        let mut occupied = [None; 100];
        for (i, id) in PIECE_IDS.iter().enumerate() {
            let (side, kind) = piece_info(id);
            let sq = INITIAL_SQUARES[i];
            pieces.push(Piece { side, kind, square: Some(sq) });
            occupied[sq as usize] = Some(i);
            index_of.insert(id.to_string(), i);
        }
        ReplayState {
            pieces,
            index_of,
            occupied,
            scores: [0, 0],
            exited_piece_count: 0,
            first_exit: None,
            ply: 0,
            current_side: 1,
            warnings: 0,
        }
    }

    fn is_over(&self) -> bool {
        self.exited_piece_count >= 2
            || matches!(self.first_exit, Some((_, PieceKind::Sentinel)))
    }

    fn apply(&mut self, m: &MoveRecord, heights: &[i16]) -> Result<(), String> {
        if self.is_over() {
            return Err(format!("coup au pli {} joué après la fin de la partie", m.ply));
        }
        if m.ply != self.ply + 1 {
            return Err(format!("pli attendu {}, reçu {}", self.ply + 1, m.ply));
        }
        if m.side != self.current_side {
            return Err(format!("camp attendu {}, reçu {}", self.current_side, m.side));
        }
        let expected_parity: u16 = if m.side == 1 { 1 } else { 0 };
        if m.ply % 2 != expected_parity {
            return Err(format!("parité incohérente : pli {} joué par le camp {}", m.ply, m.side));
        }

        if m.kind == "pass" {
            self.ply = m.ply;
            self.current_side = 3 - m.side;
            return Ok(());
        }

        let idx = *self
            .index_of
            .get(&m.piece_id)
            .ok_or_else(|| format!("pieceId inconnu : {}", m.piece_id))?;
        let (pside, pkind, pfrom) = {
            let p = &self.pieces[idx];
            (p.side, p.kind, p.square)
        };
        let from = pfrom
            .ok_or_else(|| format!("{} joue alors qu'elle est morte ou sortie", m.piece_id))?;
        if pside != m.side {
            return Err(format!("{} appartient au camp {}, coup joué par le camp {}", m.piece_id, pside, m.side));
        }
        let mfrom = m
            .from
            .ok_or_else(|| format!("coup {} sans from (pli {})", m.kind, m.ply))?;
        if from != mfrom {
            return Err(format!(
                "{} est en {} mais le coup part de {}",
                m.piece_id, from, mfrom
            ));
        }

        match m.kind.as_str() {
            "move" => {
                let to = m.to.ok_or_else(|| format!("move sans destination (pli {})", m.ply))?;
                if !(0..100).contains(&to) {
                    return Err(format!("destination hors plateau : {}", to));
                }
                if let Some(p) = &m.path {
                    if p.first() != Some(&from) || p.last() != Some(&to) {
                        return Err(format!("chemin {:?} incohérent avec {}→{}", p, from, to));
                    }
                }
                if self.occupied[to as usize].is_some() {
                    return Err(format!("case {} déjà occupée au pli {}", to, m.ply));
                }
                self.occupied[from as usize] = None;
                self.occupied[to as usize] = Some(idx);
                self.pieces[idx].square = Some(to);
                if m.special.is_none() {
                    self.soft_check_path(heights, m.path.as_deref(), pkind.budget(), 0);
                }
            }
            "attack" => {
                let to = m.to.ok_or_else(|| format!("attack sans destination (pli {})", m.ply))?;
                let cap_id = m
                    .captured_id
                    .clone()
                    .ok_or_else(|| format!("attack sans capturedId (pli {})", m.ply))?;
                if let Some(p) = &m.path {
                    if p.first() != Some(&from) || p.last() != Some(&to) {
                        return Err(format!("chemin {:?} incohérent avec {}→{}", p, from, to));
                    }
                }
                let victim = self
                    .occupied[to as usize]
                    .ok_or_else(|| format!("attack vers la case vide {} au pli {}", to, m.ply))?;
                if self.pieces[victim].side == m.side {
                    return Err(format!("capture d'une pièce de son propre camp : {}", cap_id));
                }
                if PIECE_IDS[victim] != cap_id {
                    return Err(format!(
                        "capturedId={} mais la case {} contient {}",
                        cap_id, to, PIECE_IDS[victim]
                    ));
                }
                self.pieces[victim].square = None;
                self.occupied[from as usize] = None;
                self.occupied[to as usize] = Some(idx);
                self.pieces[idx].square = Some(to);
                if m.special.is_none() {
                    self.soft_check_path(heights, m.path.as_deref(), pkind.budget(), 0);
                }
            }
            "exit" => {
                if let Some(p) = &m.path {
                    if p.first() != Some(&from) {
                        return Err(format!("chemin de sortie {:?} ne part pas de {}", p, from));
                    }
                    let last = *p.last().expect("chemin de sortie vide");
                    let on_exit_row = if m.side == 1 { (90..=99).contains(&last) } else { (0..=9).contains(&last) };
                    if !on_exit_row {
                        return Err(format!("sortie du camp {} par {} : pas sur la rangée de sortie", m.side, last));
                    }
                }
                self.occupied[from as usize] = None;
                self.pieces[idx].square = None;
                let kind = self.pieces[idx].kind;
                self.scores[(m.side - 1) as usize] += kind.value();
                self.exited_piece_count += 1;
                if self.first_exit.is_none() {
                    self.first_exit = Some((m.side, kind));
                }
                // +1 : le coût de la sortie elle-même
                if m.special.is_none() {
                    self.soft_check_path(heights, m.path.as_deref(), pkind.budget(), 1);
                }
            }
            other => return Err(format!("kind inconnu : {} (pli {})", other, m.ply)),
        }

        self.ply = m.ply;
        self.current_side = 3 - m.side;
        Ok(())
    }

    /// Contrôles SOUPLES (compteurs, pas d'échec) : budget, adjacence, montée.
    /// Si tu vois des warnings en masse, la formule de coût est à confronter
    /// à celle de ton movegen (les données suggèrent un coût de 1 par pas).
    fn soft_check_path(&mut self, heights: &[i16], path: Option<&[i16]>, budget: i32, extra: i32) {
        let Some(path) = path else { return };
        let steps = path.len().saturating_sub(1) as i32;
        if steps + extra > budget {
            self.warnings += 1;
        }
        for w in path.windows(2) {
            let (a, b) = (w[0], w[1]);
            if !is_adjacent(a, b) {
                self.warnings += 1;
            }
            if heights.len() == 100 && heights[b as usize] - heights[a as usize] > 2 {
                self.warnings += 1; // montée de plus de +2 : surprenant
            }
        }
    }
}

// ============================ Replay d'une partie ============================

fn replay_game(game: &Game) -> Result<(Vec<String>, u32), String> {
    let terrain_ok = game.terrain.heights.len() == 100;
    if !terrain_ok {
        eprintln!(
            "AVERTISSEMENT : partie {} avec {} hauteurs — exclue du dataset",
            game.id,
            game.terrain.heights.len()
        );
    }
    let mut st = ReplayState::new();
    let mut positions = Vec::new();

    for (i, m) in game.moves.iter().enumerate() {
        st.apply(m, &game.terrain.heights)?;

        // Échantillonnage (étape 2) : positions paires à partir du pli 6,
        // ni "chaudes" (capture ou sortie imminente au coup suivant) ni terminales.
        let next_tactical = game
            .moves
            .get(i + 1)
            .map_or(true, |n| n.kind == "attack" || n.kind == "exit");
        let over = st.is_over() || st.ply >= game.move_limit;
        if terrain_ok && st.ply >= 6 && st.ply % 2 == 0 && !next_tactical && !over {
            positions.push(position_line(game, &st));
        }
    }

    // -------- Validation finale contre le JSON --------
    if st.scores != game.scores {
        return Err(format!("scores rejoués {:?} ≠ JSON {:?}", st.scores, game.scores));
    }
    if st.ply != game.ply {
        return Err(format!("pli final {} ≠ JSON {}", st.ply, game.ply));
    }
    match game.reason.as_str() {
        "sentinel" => {
            let (side, kind) = st
                .first_exit
                .ok_or_else(|| "reason=sentinel mais aucune sortie rejouée".to_string())?;
            if kind != PieceKind::Sentinel {
                return Err(format!("reason=sentinel mais la première sortie est {:?}", kind));
            }
            if st.exited_piece_count != 1 {
                return Err("reason=sentinel mais plusieurs sorties rejouées".to_string());
            }
            if game.winner != Some(side) {
                return Err(format!(
                    "winner={:?} ≠ camp {} de la première sortie sentinelle",
                    game.winner, side
                ));
            }
            let ws = st.scores[(side - 1) as usize];
            let ls = st.scores[(2 - side) as usize];
            if ws != 3 || ls != 0 {
                return Err(format!(
                    "reason=sentinel mais scores {:?} (attendu 3-0 pour le camp gagnant)",
                    st.scores
                ));
            }
        }
        "score" => {
            if st.exited_piece_count != 2 {
                return Err(format!("reason=score mais {} sorties rejouées", st.exited_piece_count));
            }
            let expected = if st.scores[0] > st.scores[1] {
                Some(1u8)
            } else if st.scores[0] < st.scores[1] {
                Some(2u8)
            } else {
                None
            };
            if game.winner != expected {
                return Err(format!(
                    "winner={:?} ≠ attendu {:?} pour scores {:?}",
                    game.winner, expected, st.scores
                ));
            }
        }
        "move_limit" => {
            if st.ply != game.move_limit {
                return Err(format!(
                    "reason=move_limit mais pli final {} ≠ moveLimit {}",
                    st.ply, game.move_limit
                ));
            }
            let by_scores = if st.scores[0] > st.scores[1] {
                Some(1u8)
            } else if st.scores[0] < st.scores[1] {
                Some(2u8)
            } else {
                None
            };
            // Scores différents → le vainqueur doit les suivre.
            // Scores égaux → départage par tiebreak : accepté tel quel (non re-vérifié ici).
            if by_scores.is_some() && game.winner != by_scores {
                return Err(format!(
                    "winner={:?} contredit les scores {:?}",
                    game.winner, st.scores
                ));
            }
        }
        other => return Err(format!("reason inconnu : {}", other)),
    }
    Ok((positions, st.warnings))
}

/// Une ligne positions.jsonl = un snapshot prêt pour l'étape 2.
/// `player` : 0 = rouge au trait, 1 = bleu au trait. `label` : 1.0 / 0.5 / 0.0 (point de vue rouge).
fn position_line(game: &Game, st: &ReplayState) -> String {
    fn side_json(st: &ReplayState, side: u8) -> String {
        let (mut e, mut v, mut s) = (Vec::new(), Vec::new(), Vec::new());
        for p in &st.pieces {
            if p.side == side {
                if let Some(sq) = p.square {
                    match p.kind {
                        PieceKind::Explorer => e.push(sq),
                        PieceKind::Vanguard => v.push(sq),
                        PieceKind::Sentinel => s.push(sq),
                    }
                }
            }
        }
        format!("{{\"e\":{:?},\"v\":{:?},\"s\":{:?}}}", e, v, s)
    }
    let label = match game.winner {
        Some(1) => "1.0",
        Some(2) => "0.0",
        _ => "0.5",
    };
    let player = if st.current_side == 1 { 0 } else { 1 };
    format!(
        "{{\"game\":\"{}\",\"ply\":{},\"player\":{},\"scores\":[{},{}],\"exited\":{},\"label\":{},\"red\":{},\"blue\":{}}}",
        game.id,
        st.ply,
        player,
        st.scores[0],
        st.scores[1],
        st.exited_piece_count,
        label,
        side_json(st, 1),
        side_json(st, 2)
    )
}

// ============================ Chargement ============================

/// Accepte un fichier JSON unique ou un dossier (parse tous les *.json).
fn load_games(path: &Path) -> Vec<Game> {
    if path.is_dir() {
        let mut games = Vec::new();
        let entries = fs::read_dir(path).expect("lecture du dossier");
        for entry in entries {
            let p = entry.expect("entrée").path();
            if p.extension().map_or(false, |e| e == "json") {
                let data = fs::read_to_string(&p).expect("lecture du fichier");
                let bank: BankFile = serde_json::from_str(&data)
                    .unwrap_or_else(|e| panic!("parse de {} : {}", p.display(), e));
                eprintln!("{} : {} parties", p.display(), bank.games.len());
                games.extend(bank.games);
            }
        }
        games
    } else {
        let data = fs::read_to_string(path).expect("lecture du fichier");
        let bank: BankFile = serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("parse de {} : {}", path.display(), e));
        bank.games
    }
}

// ============================ Tests ============================

/// Partie réelle de l'export v2 (yGHLSGxi : Azalea rouge, victoire sentinelle).
/// Valide schéma, conventions d'index, plis, captures, et échantillonnage.
#[test]
fn replay_inline() {
    let json = r#"
    {"games": [{
      "id": "yGHLSGxi",
      "terrain": {"mode": "highlands", "seed": 21777282, "heights": [5,5,4,4,5,5,5,4,4,4,5,5,4,4,4,5,5,4,4,4,5,5,4,3,4,4,4,5,5,4,5,4,4,3,3,3,4,4,4,5,4,4,3,3,3,3,3,3,4,4,4,4,3,3,3,3,3,3,4,4,5,4,4,4,3,3,3,4,4,5,4,5,5,4,4,4,3,4,5,5,4,4,4,5,5,4,4,4,5,5,4,4,4,5,5,5,4,4,5,5]},
      "setup": "speed", "moveLimit": 200,
      "red": {"id": "ar8VZrfiXhNc", "name": "Azalea"},
      "blue": {"id": "bot1", "name": "Galet"},
      "side": 1, "result": "win", "winner": 1, "reason": "sentinel",
      "ply": 15, "scores": [3, 0],
      "moves": [
        {"ply": 1, "side": 1, "pieceId": "rs1", "kind": "move", "special": null, "from": 2,  "to": 32, "path": [2,12,22,32],   "notation": "", "combat": null, "capturedId": null},
        {"ply": 2, "side": 2, "pieceId": "bs1", "kind": "move", "special": null, "from": 97, "to": 67, "path": [97,87,77,67], "notation": "", "combat": null, "capturedId": null},
        {"ply": 3, "side": 1, "pieceId": "rs1", "kind": "move", "special": null, "from": 32, "to": 51, "path": [32,31,41,51], "notation": "", "combat": null, "capturedId": null},
        {"ply": 4, "side": 2, "pieceId": "bs1", "kind": "move", "special": null, "from": 67, "to": 59, "path": [67,68,58,59], "notation": "", "combat": null, "capturedId": null},
        {"ply": 5, "side": 1, "pieceId": "rs1", "kind": "move", "special": null, "from": 51, "to": 71, "path": [51,61,71],     "notation": "", "combat": null, "capturedId": null},
        {"ply": 6, "side": 2, "pieceId": "bs2", "kind": "move", "special": null, "from": 92, "to": 81, "path": [92,82,81],     "notation": "", "combat": null, "capturedId": null},
        {"ply": 7, "side": 1, "pieceId": "rs2", "kind": "move", "special": null, "from": 7,  "to": 19, "path": [7,8,9,19],    "notation": "", "combat": null, "capturedId": null},
        {"ply": 8, "side": 2, "pieceId": "bs1", "kind": "move", "special": null, "from": 59, "to": 38, "path": [59,49,48,38], "notation": "", "combat": null, "capturedId": null},
        {"ply": 9, "side": 1, "pieceId": "rv2", "kind": "move", "special": null, "from": 6,  "to": 18, "path": [6,7,8,18],    "notation": "", "combat": null, "capturedId": null},
        {"ply": 10, "side": 2, "pieceId": "bv2", "kind": "move", "special": null, "from": 93, "to": 84, "path": [93,83,84],   "notation": "", "combat": null, "capturedId": null},
        {"ply": 11, "side": 1, "pieceId": "rv2", "kind": "attack", "special": null, "from": 18, "to": 38, "path": [18,28,38], "notation": "", "combat": null, "capturedId": "bs1"},
        {"ply": 12, "side": 2, "pieceId": "bv2", "kind": "move", "special": null, "from": 84, "to": 75, "path": [84,74,75],   "notation": "", "combat": null, "capturedId": null},
        {"ply": 13, "side": 1, "pieceId": "rs1", "kind": "attack", "special": null, "from": 71, "to": 81, "path": [71,81],    "notation": "", "combat": null, "capturedId": "bs2"},
        {"ply": 14, "side": 2, "pieceId": "be1", "kind": "move", "special": null, "from": 95, "to": 84, "path": [95,85,84],  "notation": "", "combat": null, "capturedId": null},
        {"ply": 15, "side": 1, "pieceId": "rs1", "kind": "exit", "special": null, "from": 81, "to": null, "path": [81,91],    "notation": "", "combat": null, "capturedId": null}
      ]
    }]}"#;

    let bank: BankFile = serde_json::from_str(json).expect("parse du JSON embarqué");
    let game = &bank.games[0];
    let (positions, warnings) = replay_game(game).expect("replay de la partie embarquée");

    // Échantillonnage attendu : plis 6 et 8 (les plis 10/12/14 sont suivis
    // d'une capture ou d'une sortie, donc exclus comme positions "chaudes").
    assert_eq!(positions.len(), 2, "positions échantillonnées");
    assert!(positions[0].contains("\"ply\":6"), "{}", positions[0]);
    assert!(positions[1].contains("\"ply\":8"), "{}", positions[1]);
    assert_eq!(warnings, 0, "warnings de coût/adjacence sur une partie saine");
    eprintln!("positions : {}", positions.join("\n"));
}

/// Rejoue une banque ou un export complet. Échoue si UNE partie est incohérente.
#[test]
fn replay_bank() {
    let Some(path) = env::var_os("AZALEA_BANK_PATH") else {
        eprintln!("AZALEA_BANK_PATH non défini → test sauté");
        return;
    };
    let games = load_games(Path::new(&path));
    eprintln!("=== {} parties à rejouer ===", games.len());
    assert!(!games.is_empty(), "aucune partie trouvée");

    let out_dir = env::var_os("AZALEA_OUT_DIR");
    let mut games_file = None;
    let mut pos_file = None;
    if let Some(d) = &out_dir {
        fs::create_dir_all(d).expect("création du dossier de sortie");
        games_file = Some(fs::File::create(Path::new(d).join("games.jsonl")).expect("games.jsonl"));
        pos_file = Some(fs::File::create(Path::new(d).join("positions.jsonl")).expect("positions.jsonl"));
    }

    let mut ok = 0usize;
    let mut failures: Vec<String> = Vec::new();
    let mut total_positions = 0usize;
    let mut total_warnings = 0u32;

    for game in &games {
        match replay_game(game) {
            Ok((positions, warnings)) => {
                ok += 1;
                total_positions += positions.len();
                total_warnings += warnings;
                if let Some(f) = &mut games_file {
                    writeln!(
                        f,
                        "{{\"game\":\"{}\",\"mode\":\"{}\",\"seed\":{},\"heights\":{:?}}}",
                        game.id, game.terrain.mode, game.terrain.seed, game.terrain.heights
                    )
                    .unwrap();
                }
                if let Some(f) = &mut pos_file {
                    for p in positions {
                        writeln!(f, "{}", p).unwrap();
                    }
                }
            }
            Err(e) => failures.push(format!("partie {} : {}", game.id, e)),
        }
    }

    eprintln!(
        "=== {} OK / {} échecs · {} positions échantillonnées · {} warnings (coût/adjacence) ===",
        ok,
        failures.len(),
        total_positions,
        total_warnings
    );
    for f in failures.iter().take(50) {
        eprintln!("ÉCHEC : {}", f);
    }
    assert!(failures.is_empty(), "{} partie(s) incohérente(s)", failures.len());
}