use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use azalea_core::board::Board;
use azalea_core::bitboardsutils::Utils;
use azalea_core::gamestate::GameState;
use azalea_core::eval::{EvalWeights, Evaluator};

// ==========================================
// 1. STRUCTURES DE DONNÉES JSON
// ==========================================

#[derive(Debug, Deserialize, Serialize)]
pub struct GameRecord {
    pub game: String,
    pub mode: String,
    pub seed: u64,
    pub heights: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TeamPieces {
    #[serde(default)]
    pub e: Vec<i16>,
    #[serde(default)]
    pub v: Vec<i16>,
    #[serde(default)]
    pub s: Vec<i16>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PositionRecord {
    pub game: String,
    pub ply: i32,
    pub player: i32,
    pub scores: Vec<i32>,
    pub exited: i32,
    pub label: f64,
    pub red: TeamPieces,
    pub blue: TeamPieces,
}

// ==========================================
// 2. STRUCTURE D'ENTRAÎNEMENT OPTIMISÉE
// ==========================================

/// Regroupe les positions par partie pour éviter de dupliquer `Utils`
pub struct GameData {
    pub game_id: String,
    pub board: Board,
    pub utils: Utils,
    pub positions: Vec<(GameState, f64)>, // (L'état exact de la position, Le score attendu)
}

// ==========================================
// 3. FONCTION DE CHARGEMENT ET FUSION
// ==========================================

pub fn load_training_data(
    games_path: impl AsRef<Path>, 
    positions_path: impl AsRef<Path>
) -> Result<Vec<GameData>, Box<dyn std::error::Error>> {
    
    let mut games_map = HashMap::new();

    println!("Lecture des plateaux (games.txt)...");
    let file = File::open(games_path)?;
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        
        let record: GameRecord = serde_json::from_str(&line)?;
        
        // 1. Convertir les hauteurs en Board
        let mut board_arr = [0i16; 100];
        for (i, &h) in record.heights.iter().enumerate().take(100) {
            board_arr[i] = h as i16;
        }
        let board = Board { board: board_arr };
        
        // 2. Précalculer Utils pour ce plateau
        let utils = Utils::new(board);

        games_map.insert(record.game.clone(), GameData {
            game_id: record.game,
            board,
            utils,
            positions: Vec::new(),
        });
    }

    println!("Lecture des positions (positions.txt) et construction des bitboards...");
    let file = File::open(positions_path)?;
    for (line_num, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        
        let pos_rec: PositionRecord = match serde_json::from_str(&line) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Erreur ligne {}: {}", line_num + 1, e);
                continue;
            }
        };

        if let Some(game_data) = games_map.get_mut(&pos_rec.game) {
            // Reconstruire l'état bitboard complet
            let mut state = GameState {
                current_player: pos_rec.player,
                ply_count: pos_rec.ply,
                score_p1: pos_rec.scores[0],
                score_p2: pos_rec.scores[1],
                exited_piece_count: pos_rec.exited,
                explorer_p1: 0, vanguard_p1: 0, sentinel_p1: 0,
                explorer_p2: 0, vanguard_p2: 0, sentinel_p2: 0,
            };

            // Placer les pièces Rouges (Joueur 1)
            for &sq in &pos_rec.red.e { state.explorer_p1 |= 1u128 << (sq as u32); }
            for &sq in &pos_rec.red.v { state.vanguard_p1 |= 1u128 << (sq as u32); }
            for &sq in &pos_rec.red.s { state.sentinel_p1 |= 1u128 << (sq as u32); }
            
            // Placer les pièces Bleues (Joueur 2)
            for &sq in &pos_rec.blue.e { state.explorer_p2 |= 1u128 << (sq as u32); }
            for &sq in &pos_rec.blue.v { state.vanguard_p2 |= 1u128 << (sq as u32); }
            for &sq in &pos_rec.blue.s { state.sentinel_p2 |= 1u128 << (sq as u32); }

            game_data.positions.push((state, pos_rec.label));
        }
    }

    // Filtrer pour ne garder que les jeux qui ont au moins une position
    let dataset: Vec<GameData> = games_map.into_values().filter(|g| !g.positions.is_empty()).collect();
    Ok(dataset)
}

// ==========================================
// 4. LOGIQUE D'OPTIMISATION TEXEL
// ==========================================

pub fn run_texel_tuning(dataset: &[GameData]) {
    let total_positions: usize = dataset.iter().map(|g| g.positions.len()).sum();
    if total_positions == 0 {
        println!("Aucune position d'entraînement valide générée.");
        return;
    }

    let n_pos = total_positions as f64;
    println!("Début du Texel Tuning sur {} positions (issues de {} parties).", total_positions, dataset.len());
    
    let mut weights = EvalWeights::new();
    let base_learning_rate = 50000.0;
    let num_epochs = 1000;
    let k = 1.13; // Ajustement d'échelle

    // On stocke les poids en f64 pour accumuler les micro-gradients
    let mut float_weights: Vec<f64> = (0..EvalWeights::NUM_PARAMS)
        .map(|i| *weights.get_param_mut(i) as f64)
        .collect();

    for epoch in 1..=num_epochs {
        let mut total_error = 0.0;
        let mut gradients = vec![0.0; EvalWeights::NUM_PARAMS];

        for game_data in dataset {
            for (state, result) in &game_data.positions {
                // 1. Calcul de l'évaluation de base
                let eval = Evaluator::evaluate(state, &game_data.utils, &weights) as f64;
                let sigmoid = 1.0 / (1.0 + 10_f64.powf(-k * eval / 400.0));
                
                // 2. Erreur (MSE)
                let error = sigmoid - result;
                total_error += error * error; 

                // 3. Différentielle Sigmoïde
                let d_error_d_eval = 2.0 * error * sigmoid * (1.0 - sigmoid) * (k * 10_f64.ln() / 400.0);

                // 4. Différences finies pour chaque paramètre
                for i in 0..EvalWeights::NUM_PARAMS {
                    // On utilise un pas de 10 pour forcer un changement visible après la troncature en i32
                    let step = if i >= 20 { 200 } else { 10 };
                    
                    *weights.get_param_mut(i) += step; 
                    let eval_plus = Evaluator::evaluate(state, &game_data.utils, &weights) as f64;
                    *weights.get_param_mut(i) -= step; // Retour à la normale

                    // On divise par le step pour obtenir le vrai gradient unitaire
                    let d_eval_d_param = (eval_plus - eval) / (step as f64); 
                    gradients[i] += d_error_d_eval * d_eval_d_param;
                }
            }
        }

        let mse = total_error / n_pos;
        if epoch == 1 || epoch % 10 == 0 {
            println!("Epoch {:3}/{} - Erreur MSE: {:.6}", epoch, num_epochs, mse);
        }

        let learning_rate = base_learning_rate / (1.0 + 0.004 * epoch as f64); // Décroissance du taux d'apprentissage

        // 5. Mise à jour des poids flottants, puis cast en i32 pour `EvalWeights`
        for i in 0..EvalWeights::NUM_PARAMS {
            let grad = gradients[i] / n_pos;
            float_weights[i] -= learning_rate * grad;
            *weights.get_param_mut(i) = float_weights[i].round() as i32;
        }
    }

    println!("=====================================");
    println!("Optimisation terminée. Nouveaux poids :");
    for i in 0..EvalWeights::NUM_PARAMS {
        println!("Paramètre Index {:02} -> {}", i, *weights.get_param_mut(i));
    }
}

// ==========================================
// 5. POINT D'ENTRÉE DU TUNER
// ==========================================

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let games_path = r"C:\Users\amoro\Documents\Amaury\Programmation\Rust\Traversal\Plays_dataset\texel_dataset\games.txt";
    let positions_path = r"C:\Users\amoro\Documents\Amaury\Programmation\Rust\Traversal\Plays_dataset\texel_dataset\positions.txt";
    
    let dataset = load_training_data(games_path, positions_path)?;
    
    run_texel_tuning(&dataset);

    Ok(())
}