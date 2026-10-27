

pub struct Board {
    pub board : [i16; 100], // hauteur de chaque case du plateau
}

impl Board {

    #[inline]
    pub fn index_to_coordinates(index: i16) -> (i16, i16) {
        let x = index % 10;
        let y = index / 10;
        (x, y)
    }

    #[inline]
    pub fn coordinates_to_index(x: i16, y: i16) -> i16 {
        y * 10 + x
    }

    #[inline]
    pub fn get_height(&self, index: i16) -> i16 {
        self.board[index as usize]
    }

    #[inline]
    pub fn get_height(&self, x: i16, y: i16) -> i16 {
        self.board[Board::coordinates_to_index(x, y) as usize]
    }



    // utils

    pub fn get_neighbor_indices(index: i16) -> Vec<i16> {
        let mut neighbors = Vec::new();
        let (x, y) = Board::index_to_coordinates(index);

        if x > 0 {
            neighbors.push(Board::coordinates_to_index(x - 1, y));
        }
        if x < 9 {
            neighbors.push(Board::coordinates_to_index(x + 1, y));
        }
        if y > 0 {
            neighbors.push(Board::coordinates_to_index(x, y - 1));
        }
        if y < 9 {
            neighbors.push(Board::coordinates_to_index(x, y + 1));
        }

        neighbors
    }

    pub fn get_diagnonal_neighbor_indices(index: i16) -> Vec<i16> {
        let mut neighbors = Vec::new();
        let (x, y) = Board::index_to_coordinates(index);

        if x > 0 && y > 0 {
            neighbors.push(Board::coordinates_to_index(x - 1, y - 1));
        }
        if x < 9 && y > 0 {
            neighbors.push(Board::coordinates_to_index(x + 1, y - 1));
        }
        if x > 0 && y < 9 {
            neighbors.push(Board::coordinates_to_index(x - 1, y + 1));
        }
        if x < 9 && y < 9 {
            neighbors.push(Board::coordinates_to_index(x + 1, y + 1));
        }

        neighbors
    }

    // renvoie la paire (index 2 de loins, index 1 de loins) des voisins à deux cases de distance
    pub fn get_neighbors_two_steps_away(index: i16) -> Vec<(i16, i16)> {
        let mut neighbors = Vec::new();
        let (x, y) = Board::index_to_coordinates(index);

        if x > 1 {
            let neighbor_index = Board::coordinates_to_index(x - 2, y);
            let between_index = Board::coordinates_to_index(x - 1, y);
            neighbors.push((neighbor_index, between_index));
        }
        if x < 8 {
            let neighbor_index = Board::coordinates_to_index(x + 2, y);
            let between_index = Board::coordinates_to_index(x + 1, y);
            neighbors.push((neighbor_index, between_index));
        }
        if y > 1 {
            let neighbor_index = Board::coordinates_to_index(x, y - 2);
            let between_index = Board::coordinates_to_index(x, y - 1);
            neighbors.push((neighbor_index, between_index));
        }
        if y < 8 {
            let neighbor_index = Board::coordinates_to_index(x, y + 2);
            let between_index = Board::coordinates_to_index(x, y + 1);
            neighbors.push((neighbor_index, between_index));
        }

        neighbors
    }




    #[inline]
    pub fn is_neighboring_exit(index: i16, player: i16) -> bool {
        let (x, y) = Board::index_to_coordinates(index);
        match player {
            0 => y == 9,
            1 => y == 0,
            _ => false,
        }
    }



    // cree un nouveau plateau de jeu plat
    pub fn new_flat() -> Board {
        Board {
            board: [99; 100],
        }
    }



}

