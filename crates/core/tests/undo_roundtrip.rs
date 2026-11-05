//! Round-trip apply/undo : pour chaque coup légal, apply puis undo doit
//! restaurer l'état exact, bitboards ET hash Zobrist.
//!   cargo test -p azalea_core --release --test undo_roundtrip -- --nocapture

use azalea_core::board::Board;
use azalea_core::gamestate::GameState;
use azalea_core::moves::MoveGenerator;
use azalea_core::zobrist::Zobrist;
use azalea_core::bitboardsutils::Utils;

struct Rng(u64); // xorshift64, pas de crate rand
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn below(&mut self, n: u64) -> u64 { self.next() % n }
}

/// Relief aléatoire en marches ±1 (|Δh| ≤ 2), borné 0..6 — comme equivalence.rs.
fn random_heights(rng: &mut Rng) -> [i16; 100] {
    let mut h = [0i16; 100];
    let mut cur = 3i16;
    for x in 0..10 {
        h[x] = cur;
        cur = (cur + rng.below(3) as i16 - 1).clamp(0, 6);
    }
    for y in 1..10 {
        for x in 0..10 {
            let prev = h[(y - 1) * 10 + x];
            h[y * 10 + x] = (prev + rng.below(3) as i16 - 1).clamp(0, 6);
        }
    }
    h
}

#[test]
fn undo_roundtrip_sur_parties_autojouees() {
    let zob = Zobrist::new(); // Zobrist peut nécessiter d'être mut pour new_search() si utilisé
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut checked = 0u64;

    for game in 0..50 {
        let board = Board { board: random_heights(&mut rng) };
        let utils = Utils::new(board); // Si utils precalculate_all_masks demande &mut
        let gs = GameState::new();
        let mut ply = 0;

        // Si vous avez choisi la Méthode 1 (tampon partagé) :
        // let mut moves = Vec::with_capacity(128); 

        while !gs.is_game_over() && ply < 200 {
            // Ligne à ajuster selon votre choix d'optimisation pour générer les coups :
            let mut moves = Vec::with_capacity(248);
            MoveGenerator::generate_all_moves(&gs, &utils, &board, &mut moves);
            // Si Méthode 1 : MoveGenerator::generate_all_moves(&gs, &utils, &board, &mut moves);

            let before_copy = gs; // GameState implémente Copy, on sauvegarde l'état initial
            let hash_before = zob.get_hash(&before_copy);

            // itération sur les coups (utilisez .as_slice() si vous avez choisi la structure MoveList)
            for m in &moves {
                let test_state = before_copy; // On part de l'état initial
                
                // Paradigme Make/Unmake sur place
                test_state.apply_move(m);
                test_state.undo_move(m);
                
                assert_eq!(test_state, before_copy,
                    "game {game} ply {ply} : undo != état initial pour {m:?}\navant : {before_copy:?}\naprès retour : {test_state:?}");
                assert_eq!(zob.get_hash(&test_state), hash_before,
                    "game {game} ply {ply} : hash non restauré pour {m:?}");
                checked += 1;
            }

            // Choisir un coup aléatoire pour la vraie partie
            let chosen_move = moves[rng.below(moves.len() as u64) as usize];
            
            // Appliquer le coup au GameState réel de la boucle
            gs.apply_move(&chosen_move);
            ply += 1;
        }
    }
    println!("{checked} allers-retours apply/undo vérifiés");
}