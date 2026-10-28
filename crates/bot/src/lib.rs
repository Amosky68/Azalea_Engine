use convert::{key_of_azalea_move, key_of_server_move};
use azalea_engine::search::Search; 
use traversal_bot::{bot, Turn};


pub mod convert;


fn choose(turn: &Turn) -> usize {
    let gs = convert::to_gamestate(&turn.state);

    // La recherche choisit un coup azalea...
    let mv = Search.get_best_move(&gs, turn.seed);

    // ...qu'on traduit en indice de la liste serveur (rendez-vous de service).
    let key = key_of_azalea_move(&mv);
    turn.moves
        .iter()
        .position(|m| key_of_server_move(&turn.state, m) == key)
        .unwrap_or(0)
}

bot!(choose);