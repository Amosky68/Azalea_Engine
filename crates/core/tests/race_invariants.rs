//! Invariants de course (Phase 0).
//!
//! Ces tests verrouillent les propriétés qui décident les parties :
//! - une sortie de Sentinel légale est une victoire immédiate, le moteur doit la jouer ;
//! - l'éval doit être monotone quand la Sentinel se rapproche de la sortie ;
//! - le moteur ne doit pas sortir une pièce non-Sentinel quand la course est à
//!   égalité et ne se gagne qu'au tempo.

use azalea_core::bitboardsutils::Utils;
use azalea_core::board::Board;
use azalea_core::eval::{EvalWeights, Evaluator};
use azalea_core::gamestate::{GameState, Pieces};
use azalea_core::search::Search;

fn flat() -> ([i16; 100], Board, Utils) {
    let heights = [0i16; 100];
    let board = Board { board: heights };
    let utils = Utils::new(board);
    (heights, board, utils)
}

/// État vide : uniquement les pièces que le test place explicitement.
fn empty_game() -> GameState {
    let mut gs = GameState::new();
    gs.explorer_p1 = 0;
    gs.vanguard_p1 = 0;
    gs.sentinel_p1 = 0;
    gs.explorer_p2 = 0;
    gs.vanguard_p2 = 0;
    gs.sentinel_p2 = 0;
    gs.current_player = 0;
    gs.ply_count = 10;
    gs
}

#[test]
fn sortie_de_sentinel_legale_est_jouee() {
    let (_heights, board, utils) = flat();
    let mut gs = empty_game();
    // Sentinel rouge sur sa rangée de sortie (rangée 9), voisins libres.
    gs.sentinel_p1 = 1u128 << 95;
    // Sentinel bleue loin de sa sortie (rangée 0).
    gs.sentinel_p2 = 1u128 << 49;

    let mut search = Search::new(|| u64::MAX);
    search.max_depth = 2;
    let (mv, _) = search.get_best_move_with_stats(&gs, &utils, &board);

    assert!(
        mv.exit && mv.piece == Pieces::Sentinel,
        "une sortie de Sentinel légale doit être jouée immédiatement, le moteur a joué {mv:?}"
    );
}

#[test]
fn eval_monotone_sur_la_course() {
    let (_heights, _board, utils) = flat();
    let weights = EvalWeights::get_fined_tuned();

    // Même position, seule la Sentinel rouge avance d'une rangée.
    let far = {
        let mut gs = empty_game();
        gs.sentinel_p1 = 1u128 << 81; // rangée 8
        gs.sentinel_p2 = 1u128 << 59; // rangée 5
        gs
    };
    let close = {
        let mut gs = far;
        gs.sentinel_p1 = 1u128 << 91; // rangée 9, une rangée plus près
        gs
    };

    let e_far = Evaluator::evaluate(&far, &utils, &weights);
    let e_close = Evaluator::evaluate(&close, &utils, &weights);

    assert!(
        e_close > e_far,
        "rapprocher la Sentinel de la sortie doit augmenter l'éval : {e_close} <= {e_far}"
    );
}

/// Invariant Phase 1 : les deux Sentinels sortent en 4 tours, mais Rouge a le
/// trait, donc la course ne se gagne qu'au tempo. Sortir l'Explorer rapporte
/// un point et cède ce tempo, sans qu'aucune sortie n'entre dans l'horizon de
/// recherche : seule l'éval peut éviter le blunder. C'est le poids graduel de
/// progression de la Sentinel (150/rangée) qui le fait passer ; le bonus
/// décisif testé d'abord a été rejeté par SPRT (-151 Elo).
#[test]
fn pas_de_sortie_non_sentinel_si_course_a_egalite() {
    let (_heights, board, utils) = flat();
    let mut gs = empty_game();
    // Rouge au trait : Sentinel rangée 0 (4 tours), Explorer sur la rangée de
    // sortie (sortie immédiate à 1 point).
    gs.sentinel_p1 = 1u128 << 0;
    gs.explorer_p1 = 1u128 << 95;
    // Bleu : Sentinel rangée 9, également à 4 tours.
    gs.sentinel_p2 = 1u128 << 99;

    let mut search = Search::new(|| u64::MAX);
    search.max_depth = 4;
    let (mv, stats) = search.get_best_move_with_stats(&gs, &utils, &board);

    assert!(
        !mv.exit || mv.piece == Pieces::Sentinel,
        "le moteur a préféré une sortie non-Sentinel ({mv:?}, profondeur {}) alors que la course est à égalité et ne se gagne qu'au tempo",
        stats.depth
    );
}
