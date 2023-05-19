mod board;
mod cells;
mod errors;
mod minesweeper;
mod path;

use board::{check_board, get_board, print_board};
use cells::Cell;
use errors::CustomError;
use minesweeper::minesweeper;
use path::{check_path, get_path};

fn main() -> Result<(), CustomError> {
    let path: String = get_path()?;
    check_path(&path)?;

    let mut board: Vec<Vec<Cell>> = Vec::new();

    get_board(&path, &mut board)?;
    check_board(&board)?;

    println!("Input del archivo:");
    print_board(&board);

    minesweeper(&mut board);

    println!("\nOutput del programa:");
    print_board(&board);

    Ok(())
}
