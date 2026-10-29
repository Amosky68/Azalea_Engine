use crate::gamestate::{GameState, Pieces};
use crate::moves::Move;

const TABLE_BITS: usize = 18;                    // 2^18 = 262 144 entrées ≈ 8 Mo
const TABLE_SIZE: usize = 1 << TABLE_BITS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bounds { Exact, Lower, Upper }

#[derive(Debug, Clone, Copy)]
pub struct ZobristEntry {
    pub key: u64,        // clé complète : la re-vérifier au probe = anti-collision
    pub depth: i32,
    pub score: i32,
    pub best_move: Move,
    pub bound: Bounds,
}

pub struct Zobrist {
    // Clés aléatoires DÉTERMINISTES : xorshift seedé fixe — pas de hasard OS
    piece_keys: [[u64; 8]; 100],   // [case][pièce + camp*4]  (codes 1..7)
    side_key: u64,
    score_keys: [[u64; 8]; 2],      // [camp][score 0..=7]
    finished_keys: [u64; 3],        // exited_pieces_count 0..=2
    table: Vec<Option<ZobristEntry>>,
}

impl Zobrist {
    pub fn new() -> Zobrist {
        let mut s = 0x9E3779B97F4A7C15u64;
        let mut next = move || { s ^= s << 13; s ^= s >> 7; s ^= s << 17; s };

        let mut piece_keys = [[0u64; 8]; 100];
        for sq in piece_keys.iter_mut() { for k in sq.iter_mut() { *k = next(); } }
        let score_keys = [[next(), next(), next(), next(), next(), next(), next(), next()],
                          [next(), next(), next(), next(), next(), next(), next(), next()]];
        let finished_keys = [next(), next(), next()];

        Zobrist {
            piece_keys,
            side_key: next(),
            score_keys,
            finished_keys,
            table: vec![None; TABLE_SIZE],
        }
    }

    /// Clé de position : PIEÈCES + camp au trait + scores + sorties.
    /// PAS de ply_count : même position = même clé, sinon le TT ne sert à rien.
    pub fn get_hash(&self, state: &GameState) -> u64 {
        let mut h = 0u64;
        for i in 0..100 {
            let piece = state.piece_maps[i];
            if piece == Pieces::None { continue; }
            let code = piece as usize + state.player_piece_map[i] as usize * 4; // 1..7
            h ^= self.piece_keys[i][code];
        }
        if state.current_player == 1 { h ^= self.side_key; }
        h ^= self.score_keys[0][(state.scores[0] as usize).min(7)];
        h ^= self.score_keys[1][(state.scores[1] as usize).min(7)];
        h ^= self.finished_keys[(state.exited_pieces_count as usize).min(2)];
        h
    }

    pub fn probe(&self, state: &GameState) -> Option<ZobristEntry> {
        let key = self.get_hash(state);
        match &self.table[(key as usize) & (TABLE_SIZE - 1)] {
            Some(e) if e.key == key => Some(*e),   // ← re-vérification de la clé
            _ => None,
        }
    }

    /// Remplacement simple (toujours) : suffisant à cette taille.
    pub fn add(&mut self, state: &GameState, depth: i32, score: i32, mv: Move, bound: Bounds) {
        let key = self.get_hash(state);
        self.table[(key as usize) & (TABLE_SIZE - 1)] =
            Some(ZobristEntry { key, depth, score, best_move: mv, bound });
    }
}