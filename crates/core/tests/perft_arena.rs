//! Validation des règles d'azalea-core contre la banque perft de Serac
//! (générée par serac-core --features arena, elle-même validée contre le SDK).
//!
//! Convention : Serac P1 = armée rangée 9 (sort rangée 0) = azalea player 1.
//!
//!   cargo test                          → profondeurs 1-2 + divide_2 (rapide)
//!   cargo test --release -- --ignored   → profondeurs 3+ (release obligatoire)

use azalea_core::board::Board;
use azalea_core::gamestate::{GameState};
use azalea_core::moves::{MoveGenerator};
use std::collections::BTreeMap;



const PERFT_JSON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/perft_arena.json");

fn load_bank() -> serde_json::Value {
    let text = std::fs::read_to_string(PERFT_JSON)
        .expect("placez perft_arena.json dans tests/data/");
    serde_json::from_str(&text).expect("perft_arena.json malformé")
}

fn build_position(pos: &serde_json::Value) -> GameState {
    let mut gs = GameState::new(); // placement standard S-V-E-E-V-S des deux camps

    let mut heights = [0i16; 100];
    for (i, v) in pos["heights"].as_array().unwrap().iter().enumerate() {
        heights[i] = v.as_i64().expect("hauteur entière") as i16;
    }
    gs.board = Board { board: heights };

    // Serac P1 (rangée 9) = azalea player 1
    gs.current_player = match pos["side_to_move"].as_str().unwrap() {
        "P1" => 1,
        _ => 0,
    };
    gs
}



/// Perft au sens de la banque : une feuille (profondeur 0 OU partie finie) vaut 1.
fn perft(gs: &GameState, depth: u32) -> u64 {
    if depth == 0 || gs.is_game_over() {
        return 1;
    }
    let mut total = 0u64;
    for m in MoveGenerator.generate_all_moves(gs) {
        total += perft(&gs.apply_move(&m), depth - 1);
    }
    total
}

#[test]
fn profondeurs_1_2_et_divide() {
    let bank = load_bank();
    for pos in bank["positions"].as_array().unwrap() {
        let id = pos["id"].as_str().unwrap();
        let gs = build_position(pos);

        // 1. Comptes globaux aux profondeurs 1 et 2
        for d in pos["depths"].as_array().unwrap() {
            let depth = d["depth"].as_u64().unwrap() as u32;
            if depth > 2 { continue; }
            let expected = d["nodes"].as_u64().unwrap();
            let got = perft(&gs, depth);
            assert_eq!(got, expected, "{id}: perft profondeur {depth} — attendu {expected}, obtenu {got}");
        }

        // 2. divide_2 : mêmes coups racine, et même sous-compte PAR coup
        let expected: BTreeMap<(i16, i16, bool), u64> = pos["divide_2"]
            .as_array().unwrap().iter()
            .map(|e| {
                let exit = e["exit"].as_bool().unwrap();
                let from = e["from"].as_i64().unwrap() as i16;
                let to = if exit { -1 } else { e["to"].as_i64().unwrap() as i16 };
                ((from, to, exit), e["nodes"].as_u64().unwrap())
            })
            .collect();

        let actual = MoveGenerator.generate_all_moves(&gs);
        assert_eq!(actual.len(), expected.len(),
            "{id}: {} coups racine générés, {} attendus", actual.len(), expected.len());

        for m in &actual {
            let k = (m.from, m.to, m.exit);
            let exp = expected.get(&k)
                .unwrap_or_else(|| panic!("{id}: coup généré absent de la banque : {k:?}"));
            let got = perft(&gs.apply_move(m), 1);
            assert_eq!(got, *exp, "{id}: le coup {k:?} mène à {got} réponses, la banque en attend {exp}");
        }
        for k in expected.keys() {
            assert!(actual.iter().any(|m| (m.from, m.to, m.exit) == *k),
                "{id}: coup attendu mais non généré : {k:?}");
        }
    }
}

#[test]
#[ignore] // lancer avec : cargo test --release -- --ignored
fn profondeurs_profondes() {
    let bank = load_bank();
    for pos in bank["positions"].as_array().unwrap() {
        let id = pos["id"].as_str().unwrap();
        let gs = build_position(pos);
        for d in pos["depths"].as_array().unwrap() {
            let depth = d["depth"].as_u64().unwrap() as u32;
            if depth < 3 { continue; }
            let expected = d["nodes"].as_u64().unwrap();
            assert_eq!(perft(&gs, depth), expected, "{id}: perft profondeur {depth}");
        }
    }
}