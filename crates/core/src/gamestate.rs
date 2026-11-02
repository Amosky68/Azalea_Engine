use crate::moves::Move;



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pieces {
    None = 0,

    Explorer = 1,
    Vanguard = 2,
    Sentinel = 3,
}
impl Pieces {
    pub fn get_movement_budget(&self) -> i32 {
        match self {
            Pieces::Explorer => 5,
            Pieces::Vanguard => 4,
            Pieces::Sentinel => 3,
            _ => 0,
        }
    }

    pub fn get_attack_force(&self) -> i32 {
        match self {
            Pieces::Explorer => 1,
            Pieces::Vanguard => 2,
            Pieces::Sentinel => 3,
            _ => 0,
        }
    }
}



/// Représentation du gamestate à l'aide de bitboards 
/// -> Board est global, invariable durant une partie
/// 
/// current_player : 0 = player 1, 1 = player 2
/// ply_count : nombre de demi-coups joués (0..=200)
/// score_p1  : score du joueur 1 (0..=3)
/// score_p2  : score du joueur 2 (0..=3)
/// exited_pieces_count  : nombre de pièces sorties du plateau (0..=2)
/// 
/// Representation des pièces :
/// 
/// explorer_p1 : bitboard des explorers du joueur 1
/// vanguard_p1 : bitboard des vanguards du joueur 1
/// sentinel_p1 : bitboard des sentinels du joueur 1
/// explorer_p2 : bitboard des explorers du joueur 2
/// vanguard_p2 : bitboard des vanguards du joueur 2
/// sentinel_p2 : bitboard des sentinels du joueur 2










#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameState {
    pub current_player : i32, 
    pub ply_count : i32,
    pub score_p1 : i32,
    pub score_p2 : i32,
    pub exited_piece_count : i32,
    // bitboards
    pub explorer_p1 : u128, 
    pub vanguard_p1 : u128, 
    pub sentinel_p1 : u128, 

    pub explorer_p2 : u128, 
    pub vanguard_p2 : u128, 
    pub sentinel_p2 : u128, 
}



impl GameState {

    #[inline]
    pub fn new() -> GameState {
        GameState {
            current_player: 0,
            ply_count: 0,
            score_p1: 0,
            score_p2: 0,
            exited_piece_count: 0,
            explorer_p1: (1 << 4) | (1 << 5),      // carrés 4, 5
            vanguard_p1: (1 << 3) | (1 << 6),      // carrés 3, 6
            sentinel_p1: (1 << 2) | (1 << 7),      // carrés 2, 7
            explorer_p2: (1 << 94) | (1 << 95),    // carrés 94, 95 (x = 4,5 sur rangée 9)
            vanguard_p2: (1 << 93) | (1 << 96),    // carrés 93, 96
            sentinel_p2: (1 << 92) | (1 << 97),    // carrés 92, 97
        }
    }


    #[inline]
    pub fn get_piece_at(&self, index: i16) -> Pieces {
        let mask = 1_u128 << index;
        if (self.explorer_p1 | self.explorer_p2) & mask != 0 { return Pieces::Explorer; }
        if (self.vanguard_p1 | self.vanguard_p2) & mask != 0 { return Pieces::Vanguard; }
        if (self.sentinel_p1 | self.sentinel_p2) & mask != 0 { return Pieces::Sentinel; }

        Pieces::None
    }
    
    #[inline]
    pub fn is_occupied(&self, index: i16) -> bool {
        return (self.explorer_p1 | self.vanguard_p1 | self.sentinel_p1 |
                self.explorer_p2 | self.vanguard_p2 | self.sentinel_p2) & (1_u128 << index) != 0;
    }
    
    #[inline]
    pub fn is_occupied_by_ally(&self, index: i16, player: i16) -> bool {
        return (if player == 0 {
            self.explorer_p1 | self.vanguard_p1 | self.sentinel_p1
        } else {
            self.explorer_p2 | self.vanguard_p2 | self.sentinel_p2
        }) & (1_u128 << index) != 0;
    }

    #[inline]
    pub fn is_occupied_by_enemy(&self, index: i16, player: i16) -> bool {
        return self.is_occupied_by_ally(index, 1 - player);
    }

    #[inline]
    pub fn get_ally_ennemy_masks(&self) -> (u128, u128) {
        if self.current_player == 0 {
            (self.explorer_p1 | self.vanguard_p1 | self.sentinel_p1, self.explorer_p2 | self.vanguard_p2 | self.sentinel_p2)
        } else {
            (self.explorer_p2 | self.vanguard_p2 | self.sentinel_p2, self.explorer_p1 | self.vanguard_p1 | self.sentinel_p1)
        }
    }

    #[inline]
    pub fn get_ally_pieces_masks(&self) -> (u128, u128, u128) {
        if self.current_player == 0 {
            (self.explorer_p1, self.vanguard_p1, self.sentinel_p1)
        } else {
            (self.explorer_p2, self.vanguard_p2, self.sentinel_p2)
        }
    }

    #[inline]
    pub fn get_enemy_pieces_masks(&self) -> (u128, u128, u128) {
        if self.current_player == 0 {
            (self.explorer_p2, self.vanguard_p2, self.sentinel_p2)
        } else {
            (self.explorer_p1, self.vanguard_p1, self.sentinel_p1)
        }
    }


    #[inline]
    pub fn get_player_pieces_map(&self, player: i32) -> (u128, u128, u128) {
        if player == 0 {
            (self.explorer_p1 , self.vanguard_p1 , self.sentinel_p1)
        } else {
            (self.explorer_p2 , self.vanguard_p2 , self.sentinel_p2)
        }
    }





    pub fn is_game_over(&self) -> bool {
        self.exited_piece_count >= 2 || self.ply_count >= 200 || self.score_p1 >= 3 || self.score_p2 >= 3 
    }
    
    pub fn get_winner(&self) -> Option<i16> {
        // 2 pièces sont sorties 
        if self.exited_piece_count >= 2 {
            if self.score_p1 > self.score_p2 { return Some(0); }
            if self.score_p2 > self.score_p1 { return Some(1); }
        }

        // Sentinel est sortie d'abord
        if      self.score_p1 >= 3 {return Some(0); } 
        else if self.score_p2 >= 3 { return Some(1); }

        // 200 tours sont passés
        if self.ply_count >= 200 {
            if self.score_p1 > self.score_p2 { return Some(0); }
            if self.score_p2 > self.score_p1 { return Some(1); }

            return self.rows_tiebreak();
        }

        return None;
    }


    pub fn rows_tiebreak(&self) -> Option<i16> {
        // joueur qui gagne : 
        //
        // celui qui a la pièce la plus proche 
        // sinon, celui qui a la pièce la plus forte sur cette rangée égale
        // sinon, match nul

        let pieces_p1 = self.explorer_p1 | self.vanguard_p1 | self.sentinel_p1;
        let pieces_p2 = self.explorer_p2 | self.vanguard_p2 | self.sentinel_p2;

        let row = 0b1111111111_u128; 

        for distance in 0..10 {
            let row_p1 = row << ((9 - distance) * 10) & pieces_p1;
            let row_p2 = row << (distance * 10) & pieces_p2;

            if row_p1 != 0 && row_p2 != 0 {
                // les deux joueurs ont des pièces sur cette rangée
                // on compare la force de la pièce la plus forte sur cette rangée

                let strongest_p1 = if self.sentinel_p1 & row_p1 != 0 { 3 } 
                else if self.vanguard_p1 & row_p1 != 0 { 2 } 
                else if self.explorer_p1 & row_p1 != 0 { 1 } 
                else { 0 };

                let strongest_p2 = if self.sentinel_p2 & row_p2 != 0 { 3 } 
                else if self.vanguard_p2 & row_p2 != 0 { 2 } 
                else if self.explorer_p2 & row_p2 != 0 { 1 } 
                else { 0 };

                if strongest_p1 > strongest_p2 {
                    return Some(0);
                } else if strongest_p2 > strongest_p1 {
                    return Some(1);
                } else {
                    break;
                }
            } else if row_p1 != 0 {
                return Some(0);
            } else if row_p2 != 0 {
                return Some(1);
            }
        }
        None
    }


    pub fn apply_move(&self, mv: &Move) -> GameState {
        let mut next = GameState {
            current_player: self.current_player,
            ply_count: self.ply_count,
            score_p1: self.score_p1,
            score_p2: self.score_p2,
            exited_piece_count: self.exited_piece_count,
            explorer_p1: self.explorer_p1,
            vanguard_p1: self.vanguard_p1,
            sentinel_p1: self.sentinel_p1,
            explorer_p2: self.explorer_p2,
            vanguard_p2: self.vanguard_p2,
            sentinel_p2: self.sentinel_p2,
        };

        if mv.is_pass() {
            next.current_player = 1 - next.current_player;
            next.ply_count += 1;
            return next;
        }

        let piece = mv.piece;
        
        // 1. On retire la pièce de sa case de départ
        if self.current_player == 0 {
            match piece {
                Pieces::Explorer => next.explorer_p1 ^= 1_u128 << mv.from,
                Pieces::Vanguard => next.vanguard_p1 ^= 1_u128 << mv.from,
                Pieces::Sentinel => next.sentinel_p1 ^= 1_u128 << mv.from,
                _ => panic!("Invalid piece for player 1"),
            }
        } else {
            match piece {
                Pieces::Explorer => next.explorer_p2 ^= 1_u128 << mv.from,
                Pieces::Vanguard => next.vanguard_p2 ^= 1_u128 << mv.from,
                Pieces::Sentinel => next.sentinel_p2 ^= 1_u128 << mv.from,
                _ => panic!("Invalid piece for player 2"),
            }
        }

        // 2. On gère la sortie OU l'arrivée (+ capture)
        if mv.exit {
            next.exited_piece_count += 1;
            if self.current_player == 0 {
                next.score_p1 += piece.get_attack_force() as i32;
            } else {
                next.score_p2 += piece.get_attack_force() as i32;
            }
        } else {
            if mv.captured {
                let cap = 1_u128 << mv.to;
                match (mv.captured_piece, self.current_player) {
                    (Pieces::Explorer, 0) => next.explorer_p2 ^= cap,
                    (Pieces::Vanguard, 0) => next.vanguard_p2 ^= cap,
                    (Pieces::Sentinel, 0) => next.sentinel_p2 ^= cap,
                    (Pieces::Explorer, _) => next.explorer_p1 ^= cap,
                    (Pieces::Vanguard, _) => next.vanguard_p1 ^= cap,
                    (Pieces::Sentinel, _) => next.sentinel_p1 ^= cap,
                    _ => {}
                }
            }
            
            // On place la pièce sur sa case d'arrivée
            if self.current_player == 0 {
                match piece {
                    Pieces::Explorer => next.explorer_p1 |= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p1 |= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p1 |= 1_u128 << mv.to,
                    _ => {}
                }
            } else {
                match piece {
                    Pieces::Explorer => next.explorer_p2 |= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p2 |= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p2 |= 1_u128 << mv.to,
                    _ => {}
                }
            }
        }

        // Changer de joueur
        next.current_player = 1 - next.current_player;
        next.ply_count += 1;
        next
    }


    pub fn undo_move(&self, mv: &Move) -> GameState {
        let mut next = GameState {
            current_player: self.current_player,
            ply_count: self.ply_count,
            score_p1: self.score_p1,
            score_p2: self.score_p2,
            exited_piece_count: self.exited_piece_count,
            explorer_p1: self.explorer_p1,
            vanguard_p1: self.vanguard_p1,
            sentinel_p1: self.sentinel_p1,
            explorer_p2: self.explorer_p2,
            vanguard_p2: self.vanguard_p2,
            sentinel_p2: self.sentinel_p2,
        };

        if mv.is_pass() {
            next.current_player = 1 - self.current_player;
            next.ply_count -= 1;
            return next;
        }

        let piece = mv.piece;
        let mover = 1 - self.current_player;

        // 1. Remettre la pièce sur sa case de départ
        if mover == 0 {
            match piece {
                Pieces::Explorer => next.explorer_p1 |= 1_u128 << mv.from,
                Pieces::Vanguard => next.vanguard_p1 |= 1_u128 << mv.from,
                Pieces::Sentinel => next.sentinel_p1 |= 1_u128 << mv.from,
                _ => panic!("Invalid piece for player 1"),
            }
        } else {
            match piece {
                Pieces::Explorer => next.explorer_p2 |= 1_u128 << mv.from,
                Pieces::Vanguard => next.vanguard_p2 |= 1_u128 << mv.from,
                Pieces::Sentinel => next.sentinel_p2 |= 1_u128 << mv.from,
                _ => panic!("Invalid piece for player 2"),
            }
        }

        // 2. Traiter l'arrivée, la capture ou la sortie
        if mv.exit {
            next.exited_piece_count -= 1;
            if mover == 0 {
                next.score_p1 -= piece.get_attack_force() as i32;
            } else {
                next.score_p2 -= piece.get_attack_force() as i32;
            }
        } else if mv.captured {
            // Retirer la pièce attaquante de sa case d'arrivée
            if mover == 0 {
                match piece {
                    Pieces::Explorer => next.explorer_p1 ^= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p1 ^= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p1 ^= 1_u128 << mv.to,
                    _ => {}
                }
            } else {
                match piece {
                    Pieces::Explorer => next.explorer_p2 ^= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p2 ^= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p2 ^= 1_u128 << mv.to,
                    _ => {}
                }
            }

            // Remettre la pièce capturée (qui appartient au joueur actuel du pre-undo)
            if self.current_player == 0 {
                match mv.captured_piece {
                    Pieces::Explorer => next.explorer_p1 |= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p1 |= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p1 |= 1_u128 << mv.to,
                    _ => panic!("Invalid piece for player 1"),
                }
            } else {
                match mv.captured_piece {
                    Pieces::Explorer => next.explorer_p2 |= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p2 |= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p2 |= 1_u128 << mv.to,
                    _ => panic!("Invalid piece for player 2"),
                }
            }
        } else {
            // Déplacement normal : on retire la pièce de sa case d'arrivée
            if mover == 0 {
                match piece {
                    Pieces::Explorer => next.explorer_p1 ^= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p1 ^= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p1 ^= 1_u128 << mv.to,
                    _ => {}
                }
            } else {
                match piece {
                    Pieces::Explorer => next.explorer_p2 ^= 1_u128 << mv.to,
                    Pieces::Vanguard => next.vanguard_p2 ^= 1_u128 << mv.to,
                    Pieces::Sentinel => next.sentinel_p2 ^= 1_u128 << mv.to,
                    _ => {}
                }
            }
        }

        // Changer de joueur
        next.current_player = 1 - self.current_player;
        next.ply_count -= 1;
        next
    }


   

}