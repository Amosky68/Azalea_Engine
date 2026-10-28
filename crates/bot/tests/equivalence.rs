use azalea_bot::convert::{key_of_azalea_move, key_of_server_move, to_gamestate};
use azalea_core::moves::MoveGenerator;
use std::collections::BTreeSet;
use traversal_bot::core::{apply_move, legal_moves, new_game};


struct Rng(u64); // xorshift64, pas de crate rand
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn below(&mut self, n: u64) -> u64 { self.next() % n }
}

/// Terrain aléatoire en marches ±1 (|Δh| ≤ 1 ≤ MAX_STEP), borné 0..6.
fn random_heights(rng: &mut Rng) -> [u8; 100] {
    let mut h = [0u8; 100];
    let mut cur = 3i16;
    for x in 0..10 {
        h[x] = cur as u8;
        cur = (cur + rng.below(3) as i16 - 1).clamp(0, 6);
    }
    for y in 1..10 {
        for x in 0..10 {
            let prev = h[(y - 1) * 10 + x] as i16;
            h[y * 10 + x] = (prev + rng.below(3) as i16 - 1).clamp(0, 6) as u8;
        }
    }
    h
}

#[test]
fn equivalence_movegen_sur_parties_autojouees() {
    let mut rng = Rng(0x9E3779B97F4A7C15);
    for game in 0..200 {
        let mut s = new_game(random_heights(&mut rng));
        while !s.over && s.ply < 200 {
            let moves = legal_moves(&s);
            let gs = to_gamestate(&s);

            let expected: BTreeSet<_> = moves.iter()
                .map(|m| key_of_server_move(&s, m)).collect();
            let got: BTreeSet<_> = MoveGenerator::generate_all_moves(&gs)
                .iter().map(key_of_azalea_move).collect();

            assert_eq!(got, expected,
                "game {game} ply {} : coups azalea != coups serveur", s.ply);

            assert_eq!(gs.is_game_over(), s.over,
                "game {game} ply {} : is_game_over() azalea != serveur", s.ply);

            s = apply_move(&s, &moves[rng.below(moves.len() as u64) as usize]);
        }
    }
}