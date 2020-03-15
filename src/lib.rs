
mod board;

pub use board::Board;
pub use board::CellStates;
use std::os::raw::{c_uint, c_void};

#[no_mangle]
pub extern fn sudoku_new() -> *mut Board {
    let board = Box::new(Board::empty());

    Box::into_raw(board)
}

#[no_mangle]
pub unsafe extern fn sudoku_free(board: *mut Board) {
    if !board.is_null() {
        Box::from_raw(board);
    }
}

#[no_mangle]
pub extern fn sudoku_get_number(board: *mut Board, row: u32, column: u32) -> u8 {
    let board = unsafe { board.as_ref().unwrap() };

    board.cell(row as usize, column as usize).selected.unwrap_or(0)
}

#[no_mangle]
pub extern fn sudoku_generate(board: *mut Board) {
    let board = unsafe { board.as_mut().unwrap() };

    board.generate();
}

#[no_mangle]
pub extern fn sudoku_dump(board: *mut Board) {
    let board = unsafe { board.as_ref().unwrap() };

    println!("{}", board);
}

#[no_mangle]
pub extern fn sudoku_set_update_callback(board: *mut Board, ptr: *mut c_void, cb: extern fn(*mut c_void, u32, u32)) {
    let board = unsafe { board.as_mut().unwrap() };

    board.set_callback_ptr(ptr);
    board.set_update_callback(cb);
}

#[no_mangle]
pub extern fn sudoku_get_candidate(board: *mut Board, row: c_uint, column: c_uint) -> c_uint {
    let board = unsafe { board.as_ref().unwrap() };
    let mut flags: c_uint = 0;

    for candidate in board.cell(row as usize, column as usize)
        .candidate.iter() {
        flags |= (1 as c_uint) << *candidate as u32;
    }

    flags
}

#[no_mangle]
pub extern fn sudoku_get_cell_state(board: *mut Board, row: c_uint, column: c_uint) -> CellStates {
    let board = unsafe { board.as_ref().unwrap() };

    board.cell(row as usize, column as usize).states
}