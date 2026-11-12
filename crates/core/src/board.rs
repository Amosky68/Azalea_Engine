

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn get_height_xy(&self, x: i16, y: i16) -> i16 {
        self.board[Board::coordinates_to_index(x, y) as usize]
    }


    // utils
    pub fn get_neighbor_indices(&self, index: i16) -> Vec<i16> {
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

    
    pub fn get_diagnonal_neighbor_indices(&self, index: i16) -> Vec<i16> {
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


}

