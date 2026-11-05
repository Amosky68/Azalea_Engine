use std::cell::RefCell;

use azalea_core::board::Board;
use azalea_core::search::Search;
use azalea_core::bitboardsutils::Utils;
use traversal_bot::{bot, fuel_left, Turn};

pub mod convert;

/// Cache qui survit d'un coup à l'autre pendant la partie :
/// - search : conserve la TT entre les coups (le Zobrist vit dans Search)
/// - utils  : masques + sentinel_plies, recalculés seulement si le terrain change
struct Cache {
    board: Board,
    utils: Utils,
    search: Search,
}

thread_local! {
    static CACHE: RefCell<Option<Cache>> = RefCell::new(None);
}

fn choose(turn: &Turn) -> usize {
    let mut gs = convert::to_gamestate(&turn.state);
    let board = convert::to_board(&turn.state);

    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();

        // Première invocation, ou nouvelle partie (terrain différent, ou coup 0) :
        // on reconstruit tout. La TT est remise à zéro car le hash ne contient
        // pas le terrain — une position identique sur un autre terrain n'a pas
        // la même valeur.
        let new_game = match cache.as_ref() {
            None => true,
            Some(c) => c.board != board || turn.state.ply == 0,
        };
        if new_game {
            *cache = Some(Cache {
                board,
                utils: Utils::new(board),
                search: Search::new(fuel_left),
            });
        }

        let cache = cache.as_mut().unwrap();

        let fuel_start = fuel_left();
        let (mv, stats) = cache.search.get_best_move_with_stats(&mut gs, &cache.utils, &board);
        let fuel_used = fuel_start - fuel_left();

        traversal_bot::log(&format!(
            "PLY {} D {} N {} F {} TT {:.1}",
            turn.state.ply, stats.depth, stats.nodes, fuel_used,
            100.0 * stats.tt_hits as f64 / stats.tt_probes.max(1) as f64
        ));

        let key = convert::key_of_azalea_move(&mv);
        turn.moves
            .iter()
            .position(|m| convert::key_of_server_move(&turn.state, m) == key)
            .unwrap_or(0)
    })
}

bot!(choose);