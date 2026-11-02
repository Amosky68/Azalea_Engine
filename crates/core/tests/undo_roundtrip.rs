//! Round-trip apply/undo : pour chaque coup légal, apply puis undo doit
//! restaurer l'état exact, bitboards ET hash Zobrist.
//!   cargo test -p azalea_core --release --test undo_roundtrip -- --nocapture

use azalea_core::board::Board;
use azalea_core::gamestate::GameState;
use azalea_core::moves::MoveGenerator;
use azalea_core::zobrist::Zobrist;
use azalea_core::BitboardsUtils::Utils;

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
    let zob = Zobrist::new();
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut checked = 0u64;

    for game in 0..50 {
        let board = Board { board: random_heights(&mut rng) };
        let utils = Utils::new(board);
        let mut gs = GameState::new();
        let mut ply = 0;

        while !gs.is_game_over() && ply < 200 {
            let moves = MoveGenerator::generate_all_moves(&gs, &utils, &board);
            let before = gs;
            let hash_before = zob.get_hash(&before);

            for m in &moves {
                let after = before.apply_move(m);
                let back = after.undo_move(m);
                assert_eq!(back, before,
                    "game {game} ply {ply} : undo != état initial pour {m:?}\navant : {before:?}\naprès retour : {back:?}");
                assert_eq!(zob.get_hash(&back), hash_before,
                    "game {game} ply {ply} : hash non restauré pour {m:?}");
                checked += 1;
            }

            gs = before.apply_move(&moves[rng.below(moves.len() as u64) as usize]);
            ply += 1;
        }
    }
    println!("{checked} allers-retours apply/undo vérifiés");
}