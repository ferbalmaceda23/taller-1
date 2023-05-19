use buscaminas::{
    board::{check_board, get_board},
    cells::Cell,
    minesweeper::minesweeper,
    path::check_path,
};

#[test]
fn program_runs_as_expected() {
    let path: String = "src/files/buscaminas_1.txt".to_string();

    let check_path_result = check_path(&path);
    assert_eq!(check_path_result, Ok(()));

    let mut board: Vec<Vec<Cell>> = Vec::new();

    let get_board_result = get_board(&path, &mut board);
    assert_eq!(get_board_result, Ok(()));

    let check_board_result = check_board(&board);
    assert_eq!(check_board_result, Ok(()));

    minesweeper(&mut board);

    let expected_board: Vec<Vec<Cell>> = vec![
        vec![
            Cell::Empty(1),
            Cell::Mine,
            Cell::Empty(3),
            Cell::Mine,
            Cell::Empty(1),
        ],
        vec![
            Cell::Empty(1),
            Cell::Empty(3),
            Cell::Mine,
            Cell::Empty(3),
            Cell::Empty(1),
        ],
        vec![
            Cell::Empty(0),
            Cell::Empty(2),
            Cell::Mine,
            Cell::Empty(2),
            Cell::Empty(0),
        ],
        vec![
            Cell::Empty(0),
            Cell::Empty(1),
            Cell::Empty(1),
            Cell::Empty(1),
            Cell::Empty(0),
        ],
    ];

    assert_eq!(board, expected_board);
}
