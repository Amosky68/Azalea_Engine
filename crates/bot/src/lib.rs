use convert::{key_of_azalea_move, key_of_server_move};
use azalea_core::search::Search;
use traversal_bot::{bot, fuel_left, Turn};

pub mod convert;

fn choose(turn: &Turn) -> usize {
    // 1. Position serveur -> GameState azalea
    let gs = convert::to_gamestate(&turn.state);

    // 2. La recherche choisit un coup azalea ; on lui passe fuel_left du SDK
    let search = Search::new(fuel_left); 
    let mv = search.get_best_move(&gs);

    // 3. Traduction en indice de la liste serveur (rendez-vous de service)
    let key = key_of_azalea_move(&mv);
    turn.moves
        .iter()
        .position(|m| key_of_server_move(&turn.state, m) == key)
        .unwrap_or(0)
}

bot!(choose);