use azalea_core::board::Board;
use azalea_core::gamestate::{GameState, Pieces};
use traversal_bot::core::{Kind, Move as ServerMove, MoveKind, State};



/// Convertit l'état du serveur en GameState azalea.
/// Camps : owner 1 (rouge) -> player 0 ; owner 2 (bleu) -> player 1.
pub fn to_gamestate(state: &State) -> GameState {
    let mut heights = [0i16; 100];
    for (i, &h) in state.heights.iter().enumerate() {
        heights[i] = h as i16;
    }

    let mut gs = GameState {
        board: Board { board: heights },
        current_player: (state.to_move - 1) as i16,
        ply_count: state.ply as i16,
        scores: [state.scores[0] as i16, state.scores[1] as i16],
        exited_pieces_count: state.finished as i16,
        piece_maps: [Pieces::None; 100],
        player_piece_map: [0; 100],
    };

    for p in state.pieces() {
        gs.piece_maps[p.sq as usize] = match p.kind {
            Kind::Explorer => Pieces::Explorer,
            Kind::Vanguard => Pieces::Vanguard,
            Kind::Sentinel => Pieces::Sentinel,
        };
        gs.player_piece_map[p.sq as usize] = (p.owner - 1) as i16;
    }

    gs
}




/// Clé comparable d'un coup : identifie un coup par sa CASE de départ (les coups
/// serveur identifient la pièce par son INDICE dans la liste compacte, qui se
/// décale aux captures — la case est le point de rencontre naturel des deux).
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MoveKey {
    pub from: i16, // -1 pour une passe
    pub kind: u8,  // 0 move, 1 attaque, 2 sortie, 3 passe
    pub to: i16,   // -1 pour sortie/passe
}

pub fn key_of_server_move(state: &State, m: &ServerMove) -> MoveKey {
    if m.kind == MoveKind::Pass {
        return MoveKey { from: -1, kind: 3, to: -1 };
    }
    let from = state.piece(m.piece).sq as i16;
    let kind = match m.kind {
        MoveKind::Move => 0,
        MoveKind::Attack => 1,
        MoveKind::Exit => 2,
        MoveKind::Pass => 3,
    };
    // côté serveur, une sortie n'a pas de case d'arrivée (NONE) : normaliser
    let to = if m.kind == MoveKind::Exit { -1 } else { m.to as i16 };
    MoveKey { from, kind, to }
}

pub fn key_of_azalea_move(m: &azalea_core::moves::Move) -> MoveKey {
    if m.is_pass() {
        return MoveKey { from: -1, kind: 3, to: -1 };
    }
    let kind = if m.exit { 2 } else if m.captured { 1 } else { 0 };
    let to = if m.exit { -1 } else { m.to };
    MoveKey { from: m.from, kind, to }
}