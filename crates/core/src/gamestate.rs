use crate::board::Board;
use crate::moves::Move;



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pieces {
    None = 0,

    Explorer = 1,
    Vanguard = 2,
    Sentinel = 3,
}
impl Pieces {
    pub fn get_movement_budget(&self) -> i16 {
        match self {
            Pieces::Explorer => 5,
            Pieces::Vanguard => 4,
            Pieces::Sentinel => 3,
            _ => 0,
        }
    }

    pub fn get_attack_force(&self) -> i16 {
        match self {
            Pieces::Explorer => 1,
            Pieces::Vanguard => 2,
            Pieces::Sentinel => 3,
            _ => 0,
        }
    }
}





pub struct GameState {
    pub board: Board,
    pub current_player: i16, // 0 = player 1, 1 = player 2
    pub ply_count: i16,
    pub scores: [i16; 2],    // score p1, score p2
    pub exited_pieces_count: i16, // compte le nombre de pièces sorties du plateau 

    pub piece_maps: [Pieces; 100],
    pub player_piece_map: [i16; 100], // 0 = player 1, 1 = player 2
}



impl GameState {
    pub fn new() -> GameState {
        let mut gs = GameState {
            board: Board::new_flat(),
            current_player: 0,
            ply_count: 0,
            scores: [0; 2],
            exited_pieces_count: 0,

            piece_maps: [Pieces::None; 100],
            player_piece_map: [0; 100],
        };

        gs.piece_maps[Board::coordinates_to_index(2, 0) as usize] = Pieces::Sentinel;
        gs.piece_maps[Board::coordinates_to_index(3, 0) as usize] = Pieces::Vanguard;
        gs.piece_maps[Board::coordinates_to_index(4, 0) as usize] = Pieces::Explorer;
        gs.piece_maps[Board::coordinates_to_index(5, 0) as usize] = Pieces::Explorer;
        gs.piece_maps[Board::coordinates_to_index(6, 0) as usize] = Pieces::Vanguard;
        gs.piece_maps[Board::coordinates_to_index(7, 0) as usize] = Pieces::Sentinel;

        gs.piece_maps[Board::coordinates_to_index(2, 9) as usize] = Pieces::Sentinel;
        gs.piece_maps[Board::coordinates_to_index(3, 9) as usize] = Pieces::Vanguard;
        gs.piece_maps[Board::coordinates_to_index(4, 9) as usize] = Pieces::Explorer;
        gs.piece_maps[Board::coordinates_to_index(5, 9) as usize] = Pieces::Explorer;
        gs.piece_maps[Board::coordinates_to_index(6, 9) as usize] = Pieces::Vanguard;
        gs.piece_maps[Board::coordinates_to_index(7, 9) as usize] = Pieces::Sentinel;

        for i in 2..8 {
            gs.player_piece_map[Board::coordinates_to_index(i, 0) as usize] = 0;
            gs.player_piece_map[Board::coordinates_to_index(i, 9) as usize] = 1;
        }

        gs
    }

    #[inline]
    pub fn get_piece_at(&self, index: i16) -> Pieces {
        self.piece_maps[index as usize]
    }
    
    #[inline]
    pub fn is_occupied(&self, index: i16) -> bool {
        self.piece_maps[index as usize] != Pieces::None
    }
    
    #[inline]
    pub fn is_occupied_by_ally(&self, index: i16, player: i16) -> bool {
        self.player_piece_map[index as usize] == player && self.is_occupied(index)
    }

    #[inline]
    pub fn is_occupied_by_enemy(&self, index: i16, player: i16) -> bool {
        self.player_piece_map[index as usize] != player && self.is_occupied(index)
    }




    pub fn is_game_over(&self) -> bool {
        self.exited_pieces_count >= 2 || self.ply_count >= 200 || self.scores[0] >= 3 || self.scores[1] >= 3 
    }

    pub fn get_winner(&self) -> Option<i16> {
        // 2 pièces sont sorties 
        if self.exited_pieces_count >= 2 {
            if self.scores[0] > self.scores[1] { return Some(0); }
            if self.scores[1] > self.scores[0] { return Some(1); }
        }

        // Sentinel est sortie d'abord
        if      self.scores[0] >= 3 {return Some(0); } 
        else if self.scores[1] >= 3 { return Some(1); }

        // 200 tours sont passés
        if self.ply_count >= 200 {
            if self.scores[0] > self.scores[1] { return Some(0); }
            if self.scores[1] > self.scores[0] { return Some(1); }

            return self.rows_tiebreak();
        }

        return None;
    }

    pub fn rows_tiebreak(&self) -> Option<i16> {
        // best[player] = meilleure clé (rangées restantes, -force) du camp
        let mut best: [Option<(i16, i16)>; 2] = [None, None];

        for index in 0..100i16 {
            let piece = self.piece_maps[index as usize];
            if piece == Pieces::None {
                continue;
            }
            let player = self.player_piece_map[index as usize];
            let (_, y) = Board::index_to_coordinates(index);

            // player 0 sort en rangée 9 ; player 1 sort en rangée 0
            let rows_to_go = if player == 0 { 9 - y } else { y };

            // moins de rangées d'abord ; à égalité d'avance, la force la plus haute gagne
            let key = (rows_to_go, -piece.get_attack_force());

            let slot = &mut best[player as usize];
            if slot.map_or(true, |b| key < b) {
                *slot = Some(key);
            }
        }

        match best {
            [Some(a), Some(b)] if a < b => Some(0),
            [Some(a), Some(b)] if b < a => Some(1),
            [Some(_), None]            => Some(0), // camp 1 seul à avoir des pièces
            [None, Some(_)]            => Some(1),
            _                          => None,    // égalité parfaite (ou les deux vides)
        }
    }



    pub fn apply_move(&self, mv: &Move) -> GameState {
        let mut next = GameState {
            board: self.board,
            current_player: self.current_player,
            ply_count: self.ply_count,
            scores: self.scores,
            exited_pieces_count: self.exited_pieces_count,
            piece_maps: self.piece_maps,
            player_piece_map: self.player_piece_map,
        };

        if mv.is_pass() {
            next.current_player = 1 - next.current_player;
            next.ply_count += 1;
            return next;
        }


        let piece = mv.piece;
        let player = self.player_piece_map[mv.from as usize];

        if mv.exit {
            next.piece_maps[mv.from as usize] = Pieces::None;
            next.player_piece_map[mv.from as usize] = 0;

            next.exited_pieces_count += 1;
            next.scores[player as usize] += piece.get_attack_force();
        } else {
            next.piece_maps[mv.to as usize] = piece;
            next.player_piece_map[mv.to as usize] = player;

            next.piece_maps[mv.from as usize] = Pieces::None;
            next.player_piece_map[mv.from as usize] = 0;
        }

        // Changer de joueur
        next.current_player = 1 - next.current_player;
        next.ply_count += 1;
        next
    }

    pub fn undo_move(&self, mv: &Move) -> GameState {
        let mut next = GameState {
            board: self.board,
            current_player: self.current_player,
            ply_count: self.ply_count,
            scores: self.scores,
            exited_pieces_count: self.exited_pieces_count,
            piece_maps: self.piece_maps,
            player_piece_map: self.player_piece_map,
        };

        if mv.is_pass() {
            next.current_player = 1 - self.current_player;
            next.ply_count -= 1;
            return next;
        }

        let piece = mv.piece;
        let player = self.player_piece_map[mv.from as usize];

        // deplacements 
        next.piece_maps[mv.from as usize] = piece;
        next.player_piece_map[mv.from as usize] = player;

        if mv.captured {
            next.piece_maps[mv.to as usize] = mv.captured_piece;
            next.player_piece_map[mv.to as usize] = 1 - player;
        }
        else if mv.exit {
            next.exited_pieces_count -= 1;
            next.scores[player as usize] -= piece.get_attack_force();
        }
        else {
            next.piece_maps[mv.to as usize] = Pieces::None;
            next.player_piece_map[mv.to as usize] = 0;
        }
        
        next.current_player = 1 - next.current_player;
        next.ply_count -= 1;
        next
    }

}
