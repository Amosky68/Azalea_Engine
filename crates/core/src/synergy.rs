use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

// ==========================================
// 1. STRUCTURE OPTIMISÉE POUR LE MOTEUR
// ==========================================

/// Structure statique contenant les cartes de synergie (accès en O(1)).
/// Utilise des f32 pour limiter la taille mémoire à ~240 Ko.
#[derive(Debug, Clone)]
pub struct SynergyTables {
    pub sv: [[f32; 100]; 100],
    pub se: [[f32; 100]; 100],
    pub ve: [[f32; 100]; 100],
    pub ss: [[f32; 100]; 100],
    pub vv: [[f32; 100]; 100],
    pub ee: [[f32; 100]; 100],
}

impl Default for SynergyTables {
    fn default() -> Self {
        SynergyTables {
            sv: [[0.0; 100]; 100],
            se: [[0.0; 100]; 100],
            ve: [[0.0; 100]; 100],
            ss: [[0.0; 100]; 100],
            vv: [[0.0; 100]; 100],
            ee: [[0.0; 100]; 100],
        }
    }
}

// ==========================================
// 2. STRUCTURE TEMPORAIRE POUR SERDE
// ==========================================

/// Structure temporaire utilisée uniquement pendant le chargement du JSON.
#[derive(Debug, Deserialize)]
struct RawSynergyMaps {
    sv: Vec<Vec<f32>>,
    se: Vec<Vec<f32>>,
    ve: Vec<Vec<f32>>,
    ss: Vec<Vec<f32>>,
    vv: Vec<Vec<f32>>,
    ee: Vec<Vec<f32>>,
}

// ==========================================
// 3. FONCTIONS DE CHARGEMENT ET D'ACCÈS
// ==========================================

impl SynergyTables {

    /// load depuis un str Json
    pub fn load_from_str(json_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let raw: RawSynergyMaps = serde_json::from_str(json_str)?;
        let mut tables = SynergyTables::default();

        fn copy_map(src: &[Vec<f32>], dst: &mut [[f32; 100]; 100]) {
            for i in 0..100 {
                for j in 0..100 {
                    if i < src.len() && j < src[i].len() {
                        dst[i][j] = src[i][j];
                    }
                }
            }
        }

        copy_map(&raw.sv, &mut tables.sv);
        copy_map(&raw.se, &mut tables.se);
        copy_map(&raw.ve, &mut tables.ve);
        copy_map(&raw.ss, &mut tables.ss);
        copy_map(&raw.vv, &mut tables.vv);
        copy_map(&raw.ee, &mut tables.ee);

        Ok(tables)
    }


    /// Charge et convertit le fichier JSON en tableaux de taille fixe
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        // Parse le JSON de manière dynamique
        let raw: RawSynergyMaps = serde_json::from_reader(reader)?;
        let mut tables = SynergyTables::default();

        // Fonction locale pour transférer les données des Vec vers le tableau statique
        fn copy_map(src: &[Vec<f32>], dst: &mut [[f32; 100]; 100]) {
            for i in 0..100 {
                for j in 0..100 {
                    if i < src.len() && j < src[i].len() {
                        dst[i][j] = src[i][j];
                    }
                }
            }
        }

        copy_map(&raw.sv, &mut tables.sv);
        copy_map(&raw.se, &mut tables.se);
        copy_map(&raw.ve, &mut tables.ve);
        copy_map(&raw.ss, &mut tables.ss);
        copy_map(&raw.vv, &mut tables.vv);
        copy_map(&raw.ee, &mut tables.ee);

        Ok(tables)
    }

    /// Récupère la valeur de synergie entre deux pièces en gérant automatiquement
    /// l'inversion de point de vue pour le Joueur 2 (Bleu)
    #[inline]
    pub fn get_score(&self, pair_map: &[[f32; 100]; 100], sq1: i16, sq2: i16, player: i32) -> f32 {
        if sq1 < 0 || sq2 < 0 || sq1 >= 100 || sq2 >= 100 {
            return 0.0;
        }

        // Le script Python a été entraîné avec l'index "99 - sq" pour le joueur 2 (Bleu)
        let (idx1, idx2) = if player == 1 {
            (99 - sq1 as usize, 99 - sq2 as usize)
        } else {
            (sq1 as usize, sq2 as usize)
        };

        pair_map[idx1][idx2]
    }
}

// ==========================================
// 4. EXEMPLE D'UTILISATION
// ==========================================
/*
fn example_usage() {
    // À exécuter une seule fois au démarrage de votre bot
    let synergy_tables = SynergyTables::load_from_file("synergy_maps.json").unwrap();
    
    let sentinel_sq = 44;
    let vanguard_sq = 45;
    let player = 0; // Rouge
    
    // Obtenir le bonus de synergie :
    let score = synergy_tables.get_score(&synergy_tables.sv, sentinel_sq, vanguard_sq, player);
    println!("Synergie S/V : {}", score);
}
*/







pub fn get_synergy_score(state: &GameState, weights: &EvalWeights, player: i32) -> f32 {
    let mut synergy_bonus = 0.0;
    let (ally_e, ally_v, ally_s) = state.get_player_pieces_map(player as i32);



    let sentinel_1 = ally_s.trailing_zeros() as i16;
    let sentinel_2 = (ally_s & (ally_s - 1)).trailing_zeros() as i16;

    let vanguard_1 = ally_v.trailing_zeros() as i16;
    let vanguard_2 = (ally_v & (ally_v - 1)).trailing_zeros() as i16;

    let explorer_1 = ally_e.trailing_zeros() as i16;
    let explorer_2 = (ally_e & (ally_e - 1)).trailing_zeros() as i16;

    // Si il n'y a que une seul d'une sentinel par exemple, sentinel_2 sera 0 et donc sentinel_2.trailing_zeros() sera 128
    // ce qui est hors des limites du tableau et le get_score() retournera 0.0, donc pas de problème.

    synergy_bonus += synergy.get_score(&synergy.ss, sentinel_1, sentinel_2, player) * (weights.k_ss as f32);
    synergy_bonus += synergy.get_score(&synergy.vv, vanguard_1, vanguard_2, player) * (weights.k_vv as f32);
    synergy_bonus += synergy.get_score(&synergy.ee, explorer_1, explorer_2, player) * (weights.k_ee as f32);

    
    if ally_s != 0 && ally_v != 0 {
        synergy_bonus += 
        (synergy.get_score(&synergy.sv, sentinel_1, vanguard_1, player)  
        + synergy.get_score(&synergy.sv, sentinel_1, vanguard_2, player)
        + synergy.get_score(&synergy.sv, sentinel_2, vanguard_1, player)
        + synergy.get_score(&synergy.sv, sentinel_2, vanguard_2, player)) * (weights.k_sv as f32);
    }
    if ally_s != 0 && ally_e != 0 {
        synergy_bonus += 
        (synergy.get_score(&synergy.se, sentinel_1, explorer_1, player)  
        + synergy.get_score(&synergy.se, sentinel_1, explorer_2, player)
        + synergy.get_score(&synergy.se, sentinel_2, explorer_1, player)
        + synergy.get_score(&synergy.se, sentinel_2, explorer_2, player)) * (weights.k_se as f32);
    }
    if ally_v != 0 && ally_e != 0 {
        synergy_bonus += 
        (synergy.get_score(&synergy.ve, vanguard_1, explorer_1, player)  
        + synergy.get_score(&synergy.ve, vanguard_1, explorer_2, player)
        + synergy.get_score(&synergy.ve, vanguard_2, explorer_1, player)
        + synergy.get_score(&synergy.ve, vanguard_2, explorer_2, player)) * (weights.k_ve as f32);
    }

    synergy_bonus
}
