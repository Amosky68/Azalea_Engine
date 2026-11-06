use crate::board::Board;
use crate::gamestate::GameState;


const FREE_MOVEMENT_LAYOUT_MAX_BUDGETS: usize = 30; // budgets pour Explorer, Vanguard, Sentinel



pub struct Utils {
    pub bfs_weights : [[u128; 4]; 3], // direction, step size (0,1,2) 
    // dir : up, right, down, left

    pub row_mask : [u128; 10],
    pub col_mask : [u128; 10],
    pub triple_col_mask : [u128; 10], // masque des colonnes triples (pour les sentinelles)


    pub neighbor_mask : [u128; 100],
    pub diagonal_neighbor_mask : [u128; 100],
    pub two_step_mask : [u128; 100],

    // plies_to_exit[p][sq] = ⌈(dist_dijkstra(sq → rangée de sortie) + 1) / 3⌉
    // = nombre de DEMI-COUPS propres pour qu'une Sentinelle en sq sorte
    // (le +1 = le pas final hors plateau ; le /3 = budget Sentinelle par coup)
    pub sentinel_plies: [[u8; 100]; 2],
    pub dist_to_exit: [[u16; 100]; 2],

    pub free_movement_layout : [[u128; 100]; 30], // budget, index 

}

impl Utils {

    pub const P1_EXIT_MASK : u128 = 0b1111111111 << 90;
    pub const P2_EXIT_MASK : u128 = 0b1111111111;

    pub const P1_GOAL_AREA_MASK : u128 = 0b1111111111_1111111111_1111111111 << 70;
    pub const P2_GOAL_AREA_MASK : u128 = 0b1111111111_1111111111_1111111111;

    pub const TOP_SHIFT_MASK : u128 = !(0b1111111111 << 90);
    pub const BOTTOM_SHIFT_MASK : u128 = !(0b1111111111);
    // LEFT_SHIFT_MASK est utilisé avec << 1 (déplacement vers l'Est), on doit masquer la colonne 9
    pub const LEFT_SHIFT_MASK : u128 =  !(0b1000000000100000000010000000001000000000100000000010000000001000000000100000000010000000001000000000);
    // RIGHT_SHIFT_MASK est utilisé avec >> 1 (déplacement vers l'Ouest), on doit masquer la colonne 0
    pub const RIGHT_SHIFT_MASK : u128 = !(0b0000000001000000000100000000010000000001000000000100000000010000000001000000000100000000010000000001);
    pub const FIRST_COL_MASK : u128 =     0b0000000001000000000100000000010000000001000000000100000000010000000001000000000100000000010000000001;


    fn precalculate_bfs_weights(&mut self, board: &Board) {
        for layer in 0..3 {
            let mut w_up: u128 = 0;
            let mut w_right: u128 = 0;
            let mut w_down: u128 = 0;
            let mut w_left: u128 = 0;

            for index in 0..100 {
                let (x, y) = Board::index_to_coordinates(index);
                let bit = 1_u128 << index;

                if y < 9 {
                    let dh = (board.get_height_xy(x, y) - board.get_height_xy(x, y + 1)).abs();
                    if dh == layer { w_up |= bit; }
                }

                if x < 9 {
                    let dh = (board.get_height_xy(x, y) - board.get_height_xy(x + 1, y)).abs();
                    if dh == layer { w_right |= bit; }
                }

                if y > 0 {
                    let dh = (board.get_height_xy(x, y) - board.get_height_xy(x, y - 1)).abs();
                    if dh == layer { w_down |= bit; }
                }

                if x > 0 {
                    let dh = (board.get_height_xy(x, y) - board.get_height_xy(x - 1, y)).abs();
                    if dh == layer { w_left |= bit; }
                }
            }

            self.bfs_weights[layer as usize][0] = w_up;
            self.bfs_weights[layer as usize][1] = w_right;
            self.bfs_weights[layer as usize][2] = w_down;
            self.bfs_weights[layer as usize][3] = w_left;
        }
    }


    fn precalculate_row_col_masks(&mut self) {
        for i in 0..10 {
            self.row_mask[i] = 0b1111111111 << (i*10);
            self.col_mask[i] = 0b00000000010000000001000000000100000000010000000001000000000100000000010000000001 << i;
        }
    }

    fn precalculate_triple_col_masks(&mut self) {
        for i in 0..10 {
            let mut mask: u128 = 0;
            for j in 0..10 {
                if (j as i16 - i as i16).abs() <= 1 {
                    mask |= Utils::FIRST_COL_MASK << j;
                }
            }
            self.triple_col_mask[i] = mask;
        }
    }

    fn precalculate_neighbor_masks(&mut self, board : &Board) {
        for index in 0..100 {
            let mut mask : u128 = 0;
            let neighbors = board.get_neighbor_indices(index);
            for n in neighbors {
                mask |= 1 << n;
            }
            self.neighbor_mask[index as usize] = mask;
        }
    }

    fn precalculate_diagonal_neighbor_masks(&mut self, board : &Board) {
        for index in 0..100 {
            let mut mask : u128 = 0;
            let neighbors = board.get_diagnonal_neighbor_indices(index);
            for n in neighbors {
                mask |= 1 << n;
            }
            self.diagonal_neighbor_mask[index as usize] = mask;
        }
    }

    fn precalculate_two_step_masks(&mut self) {
        for index in 0..100 {
            let mut mask : u128 = 0;
            let (x,y) = Board::index_to_coordinates(index);
            if x > 1 { mask |= 1 << Board::coordinates_to_index(x-2, y); }
            if x < 8 { mask |= 1 << Board::coordinates_to_index(x+2, y); }
            if y > 1 { mask |= 1 << Board::coordinates_to_index(x, y-2); }
            if y < 8 { mask |= 1 << Board::coordinates_to_index(x, y+2); }

            self.two_step_mask[index as usize] = mask;
        }
    }



    fn bfs_from_square(&mut self, start_index: usize, mut budget : i32) -> u128 {
        let mut mask = 1_u128 << start_index;
        let mut reach = [0_u128; FREE_MOVEMENT_LAYOUT_MAX_BUDGETS];
        reach[budget as usize] = mask;
        let mut all_reach : u128 = 0;

        while  budget > 0 {
            if budget > 0 {
                let next_step = (mask & self.get_bfs_weight(0, 0)) << 10 // up
                    | (mask & self.get_bfs_weight(0, 1)) << 1   // right
                    | (mask & self.get_bfs_weight(0, 2)) >> 10  // down
                    | (mask & self.get_bfs_weight(0, 3)) >> 1;  // left  

                reach[budget as usize - 1] |= next_step;
            }

            if budget > 1 {
                let next_step = (mask & self.get_bfs_weight(1, 0)) << 10 
                    | (mask & self.get_bfs_weight(1, 1)) << 1 
                    | (mask & self.get_bfs_weight(1, 2)) >> 10 
                    | (mask & self.get_bfs_weight(1, 3)) >> 1; 

                reach[budget as usize - 2] |= next_step;
            }

            if budget > 2 {
                let next_step = (mask & self.get_bfs_weight(2, 0)) << 10 
                    | (mask & self.get_bfs_weight(2, 1)) << 1 
                    | (mask & self.get_bfs_weight(2, 2)) >> 10 
                    | (mask & self.get_bfs_weight(2, 3)) >> 1; 

                reach[budget as usize - 3] |= next_step;
            }

            budget -= 1;
            mask = reach[budget as usize];
            all_reach |= mask;
        }

        all_reach
    }

    fn precalculate_free_movement_layout(&mut self) {
        for budget in 0..FREE_MOVEMENT_LAYOUT_MAX_BUDGETS {
            for index in 0..100 {
                self.free_movement_layout[budget][index] = self.bfs_from_square(index, budget as i32);
            }
        }
    }


    /// Distances Dijkstra (coût d'arête |Δh|+1) depuis chaque case vers la rangée de sortie.
    fn dist_to_exit_row(board: &Board, exit_row: usize) -> [u16; 100] {
        const INF: u16 = u16::MAX;
        let mut dist = [INF; 100];
        for x in 0..10 {
            dist[exit_row * 10 + x] = 0;
        }

        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..100 {
                for j in board.get_neighbor_indices(i as i16) {
                    let w = (board.get_height(i as i16)
                        - board.get_height(j)).unsigned_abs() + 1;
                    let (di, dj) = (dist[i as usize], dist[j as usize]);
                    if di != INF && di + w < dj { dist[j as usize] = di + w; changed = true; }
                    if dj != INF && dj + w < di { dist[i as usize] = dj + w; changed = true; }
                }
            }
        }
        dist
    }

    fn precalculate_sentinel_plies(&mut self, board: &Board) {
        for (player, exit_row) in [(0usize, 9usize), (1usize, 0usize)] {
            let dist = Utils::dist_to_exit_row(board, exit_row);
            for sq in 0..100 {
                // plies = ⌈(dist + 1) / 3⌉ : +1 = pas final hors plateau, 3 = budget Sentinel
                self.sentinel_plies[player][sq] = ((dist[sq] + 3) / 3) as u8;
            }
        }
    }

    fn precalculate_dist_to_exit(&mut self, board: &Board) {
        for (player, exit_row) in [(0usize, 9usize), (1usize, 0usize)] {
            let dist = Utils::dist_to_exit_row(board, exit_row);
            for sq in 0..100 {
                self.dist_to_exit[player][sq] = dist[sq];
            }
        }
    }



    pub fn precalculate_all_masks(&mut self, board : &Board) {
        self.precalculate_bfs_weights(board);
        self.precalculate_row_col_masks();
        self.precalculate_triple_col_masks();
        self.precalculate_neighbor_masks(board);
        self.precalculate_diagonal_neighbor_masks(board);
        self.precalculate_two_step_masks();
        self.precalculate_sentinel_plies(board);
        self.precalculate_dist_to_exit(board);
        self.precalculate_free_movement_layout();
    }

    pub fn new(board : Board) -> Utils {
        let mut utils = Utils {
            bfs_weights : [[0; 4]; 3],
            row_mask : [0; 10],
            col_mask : [0; 10],
            triple_col_mask : [0; 10],
            neighbor_mask : [0; 100],
            diagonal_neighbor_mask : [0; 100],
            two_step_mask : [0; 100],
            sentinel_plies : [[0; 100]; 2],
            dist_to_exit : [[0; 100]; 2],
            free_movement_layout : [[0; 100]; FREE_MOVEMENT_LAYOUT_MAX_BUDGETS],
        };

        utils.precalculate_all_masks(&board);

        utils
    }



    #[inline]
    pub fn get_triple_col_mask(&self, col: usize) -> u128 {
        self.triple_col_mask[col]
    }

    #[inline]
    pub fn get_neighbor_mask(&self, index: i16) -> u128 {
        self.neighbor_mask[index as usize]
    }

    #[inline]
    pub fn get_diagonal_neighbor_mask(&self, index: i16) -> u128 {
        self.diagonal_neighbor_mask[index as usize]
    }

    #[inline]
    pub fn get_two_step_mask(&self, index: i16) -> u128 {
        self.two_step_mask[index as usize]
    }

    #[inline]
    pub fn get_row_mask(&self, row: i16) -> u128 {
        self.row_mask[row as usize]
    }

    #[inline]
    pub fn get_col_mask(&self, col: i16) -> u128 {
        self.col_mask[col as usize]
    }

    #[inline]
    pub fn get_bfs_weight(&self, layer: usize, direction: usize) -> u128 {
        self.bfs_weights[layer][direction]
    }

    #[inline]
    pub fn get_free_movement_layout(&self, budget: usize, index: usize) -> u128 {
        if index > 99 {return 0;}
        self.free_movement_layout[budget][index]
    }

    #[inline]
    pub fn get_distance_to_exit(&self, player: usize, index: usize) -> u16 {
        self.dist_to_exit[player][index]
    }


    pub fn get_best_sentinel_plies(&self, state: &GameState, player: i32) -> u32 {
        let map = if player == 0 { state.sentinel_p1 } else { state.sentinel_p2 };
        if map == 0 { return 100; }
        let table = self.sentinel_plies[player as usize];
        let mut best = 100u32;
        let mut m = map;
        while m != 0 {
            let sq = m.trailing_zeros() as usize;
            m &= m - 1;
            let p = table[sq] as u32;
            if p < best { best = p; }
        }
        best
    }
}



