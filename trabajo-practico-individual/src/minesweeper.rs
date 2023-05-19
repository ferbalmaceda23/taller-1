use super::cells::Cell;

/// Función que recorre todo el tableto en busca
/// de minas y actualiza el número de minas adyacentes
/// en cada celda vacía del tablero.
pub fn minesweeper(board: &mut Vec<Vec<Cell>>) {
    for i in 0..board.len() {
        for j in 0..board[i].len() {
            if let Cell::Mine = board[i][j] {
                continue;
            }
            let mines: u8 = look_for_mines(board, i, j);
            if mines > 0 {
                board[i][j] = Cell::Empty(mines);
            }
        }
    }
}

/// Función que determina la cantida de minas adyancentes
/// a una celda específica. Devuelve la cantidad de minas.
pub fn look_for_mines(board: &Vec<Vec<Cell>>, i: usize, j: usize) -> u8 {
    let mut mines: u8 = 0;
    for k in -1..2 {
        for l in -1..2 {
            if (i as i32 + k < 0) || (i as i32 + k >= board.len() as i32) {
                continue;
            }
            if (j as i32 + l < 0) || (j as i32 + l >= board[i].len() as i32) {
                continue;
            }
            if let Cell::Mine = board[(i as i32 + k) as usize][(j as i32 + l) as usize] {
                mines += 1;
            }
        }
    }
    mines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_with_eight_mines() {
        let board: Vec<Vec<Cell>> = vec![
            vec![Cell::Mine, Cell::Mine, Cell::Mine],
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
            vec![Cell::Mine, Cell::Mine, Cell::Mine],
        ];
        assert_eq!(8, look_for_mines(&board, 1 as usize, 1 as usize));
    }

    #[test]
    fn board_with_four_mines_on_diagonals() {
        let board: Vec<Vec<Cell>> = vec![
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
        ];
        assert_eq!(2, look_for_mines(&board, 0 as usize, 1 as usize));
        assert_eq!(2, look_for_mines(&board, 1 as usize, 0 as usize));
        assert_eq!(4, look_for_mines(&board, 1 as usize, 1 as usize));
        assert_eq!(2, look_for_mines(&board, 1 as usize, 2 as usize));
        assert_eq!(2, look_for_mines(&board, 2 as usize, 1 as usize));
    }

    #[test]
    fn board_with_four_mines_on_axes() {
        let board: Vec<Vec<Cell>> = vec![
            vec![Cell::Empty(0), Cell::Mine, Cell::Empty(0)],
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
            vec![Cell::Empty(0), Cell::Mine, Cell::Empty(0)],
        ];
        assert_eq!(2, look_for_mines(&board, 0 as usize, 0 as usize));
        assert_eq!(2, look_for_mines(&board, 0 as usize, 2 as usize));
        assert_eq!(4, look_for_mines(&board, 1 as usize, 1 as usize));
        assert_eq!(2, look_for_mines(&board, 2 as usize, 0 as usize));
        assert_eq!(2, look_for_mines(&board, 2 as usize, 2 as usize));
    }

    #[test]
    fn board_with_no_mines() {
        let board: Vec<Vec<Cell>> = vec![
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
        ];

        assert_eq!(0, look_for_mines(&board, 0 as usize, 0 as usize));
        assert_eq!(0, look_for_mines(&board, 0 as usize, 1 as usize));
        assert_eq!(0, look_for_mines(&board, 0 as usize, 2 as usize));
        assert_eq!(0, look_for_mines(&board, 1 as usize, 0 as usize));
        assert_eq!(0, look_for_mines(&board, 1 as usize, 1 as usize));
        assert_eq!(0, look_for_mines(&board, 1 as usize, 2 as usize));
        assert_eq!(0, look_for_mines(&board, 2 as usize, 0 as usize));
        assert_eq!(0, look_for_mines(&board, 2 as usize, 1 as usize));
        assert_eq!(0, look_for_mines(&board, 2 as usize, 2 as usize));
    }

    #[test]
    fn cell_in_board_with_eight_mines_correctly_updated() {
        let mut board: Vec<Vec<Cell>> = vec![
            vec![Cell::Mine, Cell::Mine, Cell::Mine],
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
            vec![Cell::Mine, Cell::Mine, Cell::Mine],
        ];

        minesweeper(&mut board);

        assert_eq!(Cell::Empty(8), board[1][1]);
    }

    #[test]
    fn cells_in_board_with_four_mines_on_diagonals_correctly_updated() {
        let mut board: Vec<Vec<Cell>> = vec![
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
        ];

        minesweeper(&mut board);

        assert_eq!(Cell::Empty(4), board[1][1]);
        assert_eq!(Cell::Empty(2), board[0][1]);
        assert_eq!(Cell::Empty(2), board[1][0]);
        assert_eq!(Cell::Empty(2), board[1][2]);
        assert_eq!(Cell::Empty(2), board[2][1]);
    }

    #[test]
    fn cells_in_board_with_four_mines_on_axes_correctly_updated() {
        let mut board: Vec<Vec<Cell>> = vec![
            vec![Cell::Empty(0), Cell::Mine, Cell::Empty(0)],
            vec![Cell::Mine, Cell::Empty(0), Cell::Mine],
            vec![Cell::Empty(0), Cell::Mine, Cell::Empty(0)],
        ];

        minesweeper(&mut board);

        assert_eq!(Cell::Empty(4), board[1][1]);
        assert_eq!(Cell::Empty(2), board[0][0]);
        assert_eq!(Cell::Empty(2), board[0][2]);
        assert_eq!(Cell::Empty(2), board[2][0]);
        assert_eq!(Cell::Empty(2), board[2][2]);
    }

    #[test]
    fn cells_in_board_with_no_mines_not_updated() {
        let mut board: Vec<Vec<Cell>> = vec![
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
            vec![Cell::Empty(0), Cell::Empty(0), Cell::Empty(0)],
        ];

        minesweeper(&mut board);

        assert_eq!(Cell::Empty(0), board[0][0]);
        assert_eq!(Cell::Empty(0), board[0][1]);
        assert_eq!(Cell::Empty(0), board[0][2]);
        assert_eq!(Cell::Empty(0), board[1][0]);
        assert_eq!(Cell::Empty(0), board[1][1]);
        assert_eq!(Cell::Empty(0), board[1][2]);
        assert_eq!(Cell::Empty(0), board[2][0]);
        assert_eq!(Cell::Empty(0), board[2][1]);
        assert_eq!(Cell::Empty(0), board[2][2]);
    }
}
