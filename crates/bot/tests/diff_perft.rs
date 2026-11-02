//! Diff movegen : azalea-core (bitboards) vs traversal_bot::core (référence SDK).
//! DFS parallèle depuis la position de départ (terrain plat) ; s'arrête au
//! premier écart et dump tout ce qu'il faut pour le diagnostiquer.
//!
//!   cargo test -p azalea_bot --release --test diff_perft -- --nocapture
//!
//! Si "aucune divergence" s'affiche alors que le perft d3 est faux,
//! passer WALK_DEPTH à 4.

use std::collections::BTreeSet;

use azalea_bot::convert::{key_of_azalea_move, key_of_server_move, MoveKey};
use azalea_core::board::Board;
use azalea_core::gamestate::GameState;
use azalea_core::moves::MoveGenerator;
use azalea_core::BitboardsUtils::Utils;
use traversal_bot::core::{apply_move, legal_moves, new_game, State};

const WALK_DEPTH: u32 = 3;

fn kind_label(kind: u8) -> &'static str {
    match kind {
        0 => "MOVE",
        1 => "ATTAQUE",
        2 => "SORTIE",
        _ => "PASSE",
    }
}

/// player 0 en haut (rangée 0), sort rangée 9.
fn sq(i: i16) -> String {
    if i < 0 { "OFF".to_string() } else { format!("(x={},y={})", i % 10, i / 10) }
}

fn dump_key(k: &MoveKey) -> String {
    format!(
        "[{:>7}] {} -> {}",
        kind_label(k.kind),
        sq(k.from),
        if k.kind == 2 { "OFF".to_string() } else { sq(k.to) }
    )
}

fn dump_grid(gs: &GameState) {
    println!("--- grille (MAJUSCULES = player 0, minuscules = player 1) ---");
    for y in 0..10 {
        let mut line = String::new();
        for x in 0..10 {
            let m = 1_u128 << (y * 10 + x);
            let c = if gs.explorer_p1 & m != 0 { 'E' }
                else if gs.vanguard_p1 & m != 0 { 'V' }
                else if gs.sentinel_p1 & m != 0 { 'S' }
                else if gs.explorer_p2 & m != 0 { 'e' }
                else if gs.vanguard_p2 & m != 0 { 'v' }
                else if gs.sentinel_p2 & m != 0 { 's' }
                else { '.' };
            line.push(c);
        }
        println!("y={y} {line}");
    }
}

fn dump_bitboards(gs: &GameState) {
    let maps = [
        ("explorer_p1", gs.explorer_p1), ("vanguard_p1", gs.vanguard_p1),
        ("sentinel_p1", gs.sentinel_p1), ("explorer_p2", gs.explorer_p2),
        ("vanguard_p2", gs.vanguard_p2), ("sentinel_p2", gs.sentinel_p2),
    ];
    for (name, bb) in maps {
        println!("--- {name} ---");
        for y in 0..10 {
            let mut line = String::new();
            for x in 0..10 {
                line.push(if bb & (1_u128 << (y * 10 + x)) != 0 { 'X' } else { '.' });
            }
            println!("{line}");
        }
    }
}

fn dump_state(gs: &GameState, sdk: &State) {
    println!("azalea  : current_player={} ply={} score=({},{}) exited={}",
        gs.current_player, gs.ply_count, gs.score_p1, gs.score_p2, gs.exited_piece_count);
    println!("serveur : to_move={} ply={} scores={:?} finished={} over={}",
        sdk.to_move, sdk.ply, sdk.scores, sdk.finished, sdk.over);
    dump_grid(gs);
    dump_bitboards(gs);
}

fn print_path(path: &[MoveKey]) {
    println!("CHEMIN depuis la racine :");
    for (i, k) in path.iter().enumerate() {
        println!("  {i}. {}", dump_key(k));
    }
}

fn walk(gs: &GameState, sdk: &State, board: &Board, utils: &Utils,
        depth: u32, path: &mut Vec<MoveKey>, nodes: &mut u64) {
    if depth == 0 { return; }
    *nodes += 1;
    if *nodes % 200_000 == 0 { println!("... {} nœuds comparés", *nodes); }

    // 0. accord sur la fin de partie (avant toute génération de coups)
    let over_az = gs.is_game_over();
    if over_az != sdk.over {
        println!("\n===== DIVERGENCE is_game_over (profondeur restante {depth}) =====");
        print_path(path);
        dump_state(gs, sdk);
        panic!("fin de partie : azalea={over_az}, serveur={}", sdk.over);
    }
    if over_az { return; } // perft : feuille

    // 1. coups azalea
    let az_moves = MoveGenerator::generate_all_moves(gs, utils, board);
    let az: BTreeSet<MoveKey> = az_moves.iter().map(key_of_azalea_move).collect();

    // 2. coups serveur
    let sv_moves = legal_moves(sdk);
    let sv: BTreeSet<MoveKey> = sv_moves.iter().map(|m| key_of_server_move(sdk, m)).collect();

    // 3. doublons internes : un Vec avec doublons gonfle le perft mais passe
    //    la comparaison d'ensembles — il faut le détecter explicitement.
    if az_moves.len() != az.len() {
        println!("\n===== DOUBLONS azalea (profondeur restante {depth}) =====");
        print_path(path);
        println!("azalea génère {} coups mais seulement {} clés distinctes :",
            az_moves.len(), az.len());
        for (i, m) in az_moves.iter().enumerate() {
            println!("  {i}. {}", dump_key(&key_of_azalea_move(m)));
        }
        dump_state(gs, sdk);
        panic!("doublons dans generate_all_moves — voir le dump ci-dessus");
    }
    if sv_moves.len() != sv.len() {
        println!("\n===== DOUBLONS serveur (profondeur restante {depth}) =====");
        print_path(path);
        dump_state(gs, sdk);
        panic!("le serveur génère {} coups pour {} clés — inattendu",
            sv_moves.len(), sv.len());
    }

    // 4. comparaison des ensembles
    if az != sv {
        println!("\n===== DIVERGENCE MOVEGEN (profondeur restante {depth}) =====");
        print_path(path);
        println!("\nCOUPS EN TROP (azalea les génère, pas le serveur) :");
        for k in az.difference(&sv) { println!("  {}", dump_key(k)); }
        println!("COUPS MANQUANTS (le serveur les génère, pas azalea) :");
        for k in sv.difference(&az) { println!("  {}", dump_key(k)); }
        println!("\nTOUS les coups azalea ({}) :", az.len());
        for k in &az { println!("  {}", dump_key(k)); }
        println!("TOUS les coups serveur ({}) :", sv.len());
        for k in &sv { println!("  {}", dump_key(k)); }
        println!();
        dump_state(gs, sdk);
        panic!("divergence movegen — voir le dump ci-dessus");
    }

    // 5. recurse sur chaque coup commun
    for key in az.clone() {
        let mv_az = az_moves.iter().find(|m| key_of_azalea_move(m) == key).unwrap();
        let mv_sv = sv_moves.iter().find(|m| key_of_server_move(sdk, m) == key).unwrap();

        let gs2 = gs.apply_move(mv_az);
        let sdk2 = apply_move(sdk, mv_sv);

        path.push(key);
        walk(&gs2, &sdk2, board, utils, depth - 1, path, nodes);
        path.pop();
    }
}

#[test]
fn diff_flat_d3() {
    // Terrain plat : toutes hauteurs égales (0). Seuls les |Δh| comptent,
    // donc c'est sémantiquement identique au "start-flat" de la banque perft.
    let board = Board { board: [0i16; 100] };
    let utils = Utils::new(board);

    let gs = GameState::new();      // placement standard, player 0 au trait
    let sdk = new_game([0u8; 100]); // owner 1 = player 0, joue premier

    assert_eq!(gs.current_player as i64, sdk.to_move as i64 - 1,
        "le camp au trait initial ne coïncide pas");

    let mut path = Vec::new();
    let mut nodes = 0u64;
    walk(&gs, &sdk, &board, &utils, WALK_DEPTH, &mut path, &mut nodes);
    println!("aucune divergence à profondeur {WALK_DEPTH} ({} nœuds comparés)", nodes);
}