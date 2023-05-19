use super::cells::Cell;
use super::errors::CustomError;
use std::fs;

/// Función que chequea que el tablero inicializado
/// sea correcto. En caso de que no lo sea, devuelve
/// un error del tipo CustomError::EmptyBoard o
/// CustomError::InvalidBoard.
pub fn check_board(board: &Vec<Vec<Cell>>) -> Result<(), CustomError> {
    if board.is_empty() {
        println!("Error, el tablero esta vacio");
        return Err(CustomError::EmptyBoard);
    }
    let row_len = board[0].len();
    for row in board {
        if row.len() != row_len {
            println!("Error en el tamaño de las filas del tablero");
            return Err(CustomError::InvalidBoard);
        }
    }
    Ok(())
}

/// Función que inicializa el tablero leyendo el archivo
/// de entrada y cargando los distintas celdas segun
/// el caracter leido. En caso de que el caracter leido
/// no sea un '*' o un '.', devuelve un error del tipo
/// CustomError::InvalidChar.
pub fn get_board(path: &String, board: &mut Vec<Vec<Cell>>) -> Result<(), CustomError> {
    let data = fs::read_to_string(path)?;
    data.lines().for_each(|line| {
        let cells: Vec<Cell> = line
            .as_bytes()
            .iter()
            .map(|&x| match x as char {
                '*' => Cell::Mine,
                '.' => Cell::Empty(0),
                _ => Cell::Invalid,
            })
            .collect();
        if cells.contains(&Cell::Invalid) {
            return;
        }
        board.push(cells);
    });
    if data.lines().count() != board.len() {
        println!("Error, el archivo contiene caracteres invalidos");
        return Err(CustomError::InvalidChar);
    }
    Ok(())
}

/// Funcion que imprime por consola el tablero del buscaminas.
pub fn print_board(board: &Vec<Vec<Cell>>) {
    for row in board {
        for cell in row {
            print!("{}", cell);
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properly_formatted_file_gives_correct_board() {
        let mut board: Vec<Vec<Cell>> = Vec::new();
        let correct_board: Vec<Vec<Cell>> = vec![
            vec![
                Cell::Empty(0),
                Cell::Mine,
                Cell::Empty(0),
                Cell::Mine,
                Cell::Empty(0),
            ],
            vec![
                Cell::Empty(0),
                Cell::Empty(0),
                Cell::Mine,
                Cell::Empty(0),
                Cell::Empty(0),
            ],
            vec![
                Cell::Empty(0),
                Cell::Empty(0),
                Cell::Mine,
                Cell::Empty(0),
                Cell::Empty(0),
            ],
            vec![
                Cell::Empty(0),
                Cell::Empty(0),
                Cell::Empty(0),
                Cell::Empty(0),
                Cell::Empty(0),
            ],
        ];

        let path = "src/files/buscaminas_1.txt".to_string();
        let result = get_board(&path, &mut board);

        assert_eq!(board, correct_board);
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn invalid_char_in_file_raise_error() {
        let mut board: Vec<Vec<Cell>> = Vec::new();
        let path = "src/files/test_file_1.txt".to_string();
        let result = get_board(&path, &mut board);
        assert_eq!(result, Err(CustomError::InvalidChar));
    }

    #[test]
    fn different_row_len_in_file_raise_error() {
        let mut board: Vec<Vec<Cell>> = Vec::new();
        let path = "src/files/test_file_2.txt".to_string();

        match get_board(&path, &mut board) {
            Ok(_) => {}
            Err(_) => {}
        };

        let result = check_board(&mut board);
        assert_eq!(result, Err(CustomError::InvalidBoard));
    }

    #[test]
    fn empty_file_raise_error() {
        let mut board: Vec<Vec<Cell>> = Vec::new();
        let path = "src/files/test_file_3.txt".to_string();

        match get_board(&path, &mut board) {
            Ok(_) => {}
            Err(_) => {}
        };

        let result = check_board(&mut board);
        assert_eq!(result, Err(CustomError::EmptyBoard));
    }
}
