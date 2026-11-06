//! Audit de course (Phase 0).
//!
//! Rejoue un export de parties de l'arène, fait choisir le moteur à chaque
//! trait du bot analysé, et signale les « blunders de course » : sortir une
//! pièce non-Sentinel à égalité de course (my == opp), ce qui cède le tempo
//! qui la gagnait. Une sortie qui atteint 3 points gagne immédiatement et
//! n'est pas un blunder.
//!
//!   cargo run -p azalea_core --release --bin race_bench -- \
//!       --file Plays_dataset/traversal-arena-azalea-v2.json --depth 6
//!
//! Options : --max-games N, --every N, --side 1|2, --verbose.
//! `--dump-game ID --dump-ply N` imprime la position en fixture Rust pour un test.
//! Le chemin par défaut est résolu depuis la racine du workspace.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::env;
use std::fs;

use azalea_core::bitboardsutils::Utils;
use azalea_core::board::Board;
use azalea_core::eval::{EvalWeights, Evaluator};
use azalea_core::gamestate::{GameState, Pieces};
use azalea_core::moves::{Move, MoveGenerator};
use azalea_core::search::Search;
use serde_json::Value;

const INF_TURNS: i32 = 99;

struct Args {
    file: String,
    depth: usize,
    every: usize,
    max_games: usize,
    side: Option<i32>,
    verbose: bool,
    dump_game: Option<String>,
    dump_ply: Option<u64>,
    race_win: Option<i32>,
}

impl Args {
    fn parse() -> Args {
        let mut a = Args {
            file: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../Plays_dataset/traversal-arena-azalea-v2.json"
            )
            .to_string(),
            depth: 6,
            every: 1,
            max_games: 0,
            side: None,
            verbose: false,
            dump_game: None,
            dump_ply: None,
            race_win: None,
        };
        let mut it = env::args().skip(1);
        while let Some(k) = it.next() {
            match k.as_str() {
                "--file" => a.file = it.next().expect("--file PATH"),
                "--depth" => a.depth = it.next().expect("--depth N").parse().expect("depth"),
                "--every" => a.every = it.next().expect("--every N").parse().expect("every"),
                "--max-games" => {
                    a.max_games = it.next().expect("--max-games N").parse().expect("max-games")
                }
                "--side" => a.side = Some(it.next().expect("--side 1|2").parse().expect("side")),
                "--verbose" => a.verbose = true,
                "--dump-game" => {
                    a.dump_game = Some(it.next().expect("--dump-game ID"))
                }
                "--dump-ply" => {
                    a.dump_ply = Some(it.next().expect("--dump-ply N").parse().expect("dump-ply"))
                }
                "--race-win" => {
                    a.race_win = Some(it.next().expect("--race-win N").parse().expect("race-win"))
                }
                "-h" | "--help" => {
                    println!(
                        "usage: race_bench [--file PATH] [--depth N] [--every N] [--max-games N] [--side 1|2] [--verbose] [--race-win N] [--dump-game ID --dump-ply N]"
                    );
                    std::process::exit(0);
                }
                other => panic!("argument inconnu : {other}"),
            }
        }
        a
    }
}

#[derive(Default)]
struct Totals {
    games: usize,
    positions: usize,
    blunders: usize,
    played_blunders: usize,
    missed: usize,
    replays: usize,
}

fn all_pieces(gs: &GameState) -> u128 {
    gs.explorer_p1
        | gs.vanguard_p1
        | gs.sentinel_p1
        | gs.explorer_p2
        | gs.vanguard_p2
        | gs.sentinel_p2
}

/// Coût minimal (|dh|+1) pour atteindre la rangée de sortie, puis +1 pour le
/// pas final hors plateau, converti en nombre de tours (budget Sentinel = 3).
fn sentinel_turns(hs: &[i16; 100], occ: u128, sq: usize, exit_row: usize) -> i32 {
    let mut dist = [u16::MAX; 100];
    let mut pq: BinaryHeap<Reverse<(u16, usize)>> = BinaryHeap::new();
    dist[sq] = 0;
    pq.push(Reverse((0, sq)));

    while let Some(Reverse((c, i))) = pq.pop() {
        if c != dist[i] {
            continue;
        }
        let x = i % 10;
        let y = i / 10;
        let neighbors = [
            if x > 0 { Some(i - 1) } else { None },
            if x < 9 { Some(i + 1) } else { None },
            if y > 0 { Some(i - 10) } else { None },
            if y < 9 { Some(i + 10) } else { None },
        ];
        for j in neighbors.into_iter().flatten() {
            // La pièce ne bloque pas sa propre case de départ.
            if j != sq && (occ >> j) & 1 != 0 {
                continue;
            }
            let w = (hs[j] - hs[i]).unsigned_abs() as u16 + 1;
            let nc = c + w;
            if nc < dist[j] {
                dist[j] = nc;
                pq.push(Reverse((nc, j)));
            }
        }
    }

    let mut best = u16::MAX;
    for x in 0..10 {
        let d = dist[exit_row * 10 + x];
        if d < best {
            best = d;
        }
    }
    if best == u16::MAX {
        INF_TURNS
    } else {
        (best as i32 + 1 + 2) / 3
    }
}

fn race_turns(gs: &GameState, hs: &[i16; 100], player: i32) -> i32 {
    let map = if player == 0 {
        gs.sentinel_p1
    } else {
        gs.sentinel_p2
    };
    if map == 0 {
        return INF_TURNS;
    }
    let occ = all_pieces(gs);
    let exit_row = if player == 0 { 9 } else { 0 };
    let mut best = INF_TURNS;
    let mut m = map;
    while m != 0 {
        let sq = m.trailing_zeros() as usize;
        m &= m - 1;
        let t = sentinel_turns(hs, occ, sq, exit_row);
        if t < best {
            best = t;
        }
    }
    best
}

fn move_from_json(gs: &GameState, m: &Value) -> Move {
    let kind = m["kind"].as_str().unwrap_or("pass");
    if kind == "pass" {
        return Move::pass_mv();
    }
    let piece = match m["pieceId"].as_str().unwrap_or("").as_bytes().get(1) {
        Some(b'e') => Pieces::Explorer,
        Some(b'v') => Pieces::Vanguard,
        Some(b's') => Pieces::Sentinel,
        _ => Pieces::None,
    };
    let from = m["from"].as_i64().unwrap_or(-1) as i16;
    let to = m["to"].as_i64().unwrap_or(-1) as i16;
    let captured = kind == "attack";
    let captured_piece = if captured {
        gs.get_piece_at(to)
    } else {
        Pieces::None
    };
    Move {
        from,
        to,
        piece,
        captured_piece,
        captured,
        exit: kind == "exit",
    }
}

fn move_label(m: &Move) -> String {
    if m.is_pass() {
        return "pass".to_string();
    }
    let p = match m.piece {
        Pieces::Explorer => 'E',
        Pieces::Vanguard => 'V',
        Pieces::Sentinel => 'S',
        _ => '?',
    };
    if m.exit {
        format!("exit:{p}@{}", m.from)
    } else if m.captured {
        format!("attack:{p}@{}x{}", m.from, m.to)
    } else {
        format!("move:{p}@{}>{}", m.from, m.to)
    }
}

fn immediate_sentinel_exit(gs: &GameState, utils: &Utils, board: &Board) -> bool {
    let mut buf = [Move::pass_mv(); 256];
    let mut n = 0usize;
    MoveGenerator::generate_all_moves(gs, utils, board, &mut buf, &mut n);
    buf[..n]
        .iter()
        .any(|m| m.exit && m.piece == Pieces::Sentinel)
}

#[allow(clippy::too_many_arguments)]
fn analyze(
    totals: &mut Totals,
    details: &mut Vec<(i32, String)>,
    args: &Args,
    gid: &str,
    ply: u64,
    gs: &GameState,
    utils: &Utils,
    board: &Board,
    heights: &[i16; 100],
    search: &mut Search,
    weights: &EvalWeights,
    played: &Move,
) {
    let my = race_turns(gs, heights, gs.current_player);
    let opp = race_turns(gs, heights, 1 - gs.current_player);
    // La course n'a de sens que si les deux Sentinels sont encore en vie.
    let race_alive = my < INF_TURNS && opp < INF_TURNS;
    let tempo_ok = race_alive && my <= opp;

    if args.dump_game.as_deref() == Some(gid) && args.dump_ply == Some(ply) {
        dump_position(gs, heights, my, opp);
        std::process::exit(0);
    }

    let (chosen, stats) = search.get_best_move_with_stats(gs, utils, board);
    let eval = Evaluator::evaluate(gs, utils, weights);
    let immediate = immediate_sentinel_exit(gs, utils, board);

    // À égalité de course, seul le trait départage : une sortie non-Sentinel
    // consomme ce tempo et rend la course perdante. Si elle atteint 3 points,
    // elle gagne immédiatement et n'est pas un blunder.
    let my_points = if gs.current_player == 0 { gs.score_p1 } else { gs.score_p2 };
    let exit_wins = |m: &Move| {
        m.exit && my_points + m.piece.get_attack_force() as i32 >= 3
    };
    let blunder = chosen.exit
        && chosen.piece != Pieces::Sentinel
        && !exit_wins(&chosen)
        && tempo_ok
        && my == opp;
    let played_blunder = played.exit
        && played.piece != Pieces::Sentinel
        && !exit_wins(played)
        && tempo_ok
        && my == opp;
    let missed = immediate && !(chosen.exit && chosen.piece == Pieces::Sentinel);

    totals.positions += 1;
    if blunder {
        totals.blunders += 1;
    }
    if played_blunder {
        totals.played_blunders += 1;
    }
    if missed {
        totals.missed += 1;
    }

    if args.verbose || blunder || missed {
        let tag = if missed {
            "MISSED_WIN"
        } else if blunder {
            "BLUNDER"
        } else {
            "ok"
        };
        let line = format!(
            "g={gid} ply={ply} my={my} opp={opp} eval={eval} played={} chosen={} depth={} nodes={} [{tag}]",
            move_label(played),
            move_label(&chosen),
            stats.depth,
            stats.nodes
        );
        println!("{line}");
        if blunder || missed {
            details.push((if missed { 1000 } else { 500 } + (opp - my), line));
        }
    }
}

fn dump_position(gs: &GameState, hs: &[i16; 100], my: i32, opp: i32) {
    println!("// course : my={my} opp={opp} (tours)");
    println!("let mut gs = GameState::new();");
    println!("gs.current_player = {};", gs.current_player);
    println!("gs.ply_count = {};", gs.ply_count);
    println!("gs.score_p1 = {};", gs.score_p1);
    println!("gs.score_p2 = {};", gs.score_p2);
    println!("gs.exited_piece_count = {};", gs.exited_piece_count);
    println!("gs.explorer_p1 = 0x{:032x};", gs.explorer_p1);
    println!("gs.vanguard_p1 = 0x{:032x};", gs.vanguard_p1);
    println!("gs.sentinel_p1 = 0x{:032x};", gs.sentinel_p1);
    println!("gs.explorer_p2 = 0x{:032x};", gs.explorer_p2);
    println!("gs.vanguard_p2 = 0x{:032x};", gs.vanguard_p2);
    println!("gs.sentinel_p2 = 0x{:032x};", gs.sentinel_p2);
    println!("let heights: [i16; 100] = {hs:?};");
}

fn main() {
    let args = Args::parse();
    let text = fs::read_to_string(&args.file)
        .unwrap_or_else(|e| panic!("lecture {} : {e}", args.file));
    let data: Value = serde_json::from_str(&text).expect("JSON invalide");
    let games = data["games"].as_array().expect("games[]");
    let mut weights = EvalWeights::get_fined_tuned();
    if let Some(w) = args.race_win {
        weights.race_win = w;
    }

    let mut totals = Totals::default();
    let mut details: Vec<(i32, String)> = Vec::new();

    for (gi, g) in games.iter().enumerate() {
        if args.max_games > 0 && gi >= args.max_games {
            break;
        }
        totals.games += 1;

        let heights_v = g["terrain"]["heights"].as_array().expect("heights[]");
        let mut heights = [0i16; 100];
        for (i, v) in heights_v.iter().enumerate() {
            heights[i] = v.as_i64().unwrap_or(0) as i16;
        }
        let board = Board { board: heights };
        let utils = Utils::new(board);

        let az_side = args
            .side
            .unwrap_or_else(|| g["side"].as_i64().unwrap_or(1) as i32);
        let gid = g["id"].as_str().unwrap_or("?").to_string();

        let mut gs = GameState::new();
        let mut search = Search::new(|| u64::MAX);
        search.max_depth = args.depth;

        let mut turn_index = 0usize;
        let moves = g["moves"].as_array().expect("moves[]");
        for m in moves {
            let played = move_from_json(&gs, m);
            if gs.current_player + 1 == az_side {
                turn_index += 1;
                if turn_index % args.every == 0 {
                    analyze(
                        &mut totals,
                        &mut details,
                        &args,
                        &gid,
                        m["ply"].as_u64().unwrap_or(0),
                        &gs,
                        &utils,
                        &board,
                        &heights,
                        &mut search,
                        &weights,
                        &played,
                    );
                }
            }
            gs = gs.apply_move(&played);
        }

        if let Some(sc) = g["scores"].as_array() {
            let s1 = sc.first().and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
            let s2 = sc.get(1).and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
            if gs.score_p1 != s1 || gs.score_p2 != s2 {
                totals.replays += 1;
                eprintln!(
                    "REPLAY {gid} : scores attendus {s1}-{s2}, obtenus {}-{}",
                    gs.score_p1, gs.score_p2
                );
            }
        }
    }

    details.sort_by_key(|(k, _)| Reverse(*k));
    if !details.is_empty() {
        println!("\n== pires cas ==");
        for (_, line) in details.iter().take(10) {
            println!("{line}");
        }
    }

    println!("\n== résumé ==");
    println!("parties={} positions={}", totals.games, totals.positions);
    println!(
        "blunders moteur (sortie non-Sentinel qui cède une course à égalité) = {} ({:.2} %)",
        totals.blunders,
        100.0 * totals.blunders as f64 / totals.positions.max(1) as f64
    );
    println!("blunders joués dans l'arène = {}", totals.played_blunders);
    println!("victoires immédiates manquées = {}", totals.missed);
    println!("replays incohérents = {}", totals.replays);
}
