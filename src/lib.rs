
mod board;
mod cell;

pub use board::Board;
pub use cell::CellStates;
use std::os::raw::c_void;

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

    board.cell(row as usize, column as usize).selected().unwrap_or(0)
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
pub extern fn sudoku_get_candidate(board: *mut Board, row: u32, column: u32) -> u32 {
    let board = unsafe { board.as_ref().unwrap() };

    board.cell(row as usize, column as usize).candidate_u32()
}

#[no_mangle]
pub extern fn sudoku_get_cell_state(board: *mut Board, row: u32, column: u32) -> CellStates {
    let board = unsafe { board.as_ref().unwrap() };

    board.cell(row as usize, column as usize).states()
}

#[no_mangle]
pub extern fn sudoku_set_cell(board: *mut Board, row: u32, column: u32, val: u8) {
    let board = unsafe { board.as_mut().unwrap() };

    board.set(row as usize, column as usize, Some(val));
}
