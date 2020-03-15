
use rand;

use std::fmt;
use std::collections::HashSet;
use std::os::raw::c_void;
use std::ptr::null_mut;
use rand::Rng;
use bitflags::bitflags;
use bitflags::_core::ptr::null;

bitflags! {
    #[repr(C)]
    pub struct CellStates : u32 {
        // None
        const NONE          = 0b00000000;
        // 已经有数据的格子
        const FILLED        = 0b00000001;
        // 选中的格子
        const SELECTED      = 0b00000010;
        // 正在检查的数字所在的格子
        const CHECKING      = 0b00000100;
        // 高亮提示格子
        const HIGH_LIGHT    = 0b00001000;
        // 冲突格子
        const CONFLICT      = 0b00010000;
        // 预先定义的格子
        const PRE_FILLED    = 0b00100001;
    }
}

#[derive(Clone)]
pub struct Cell {
    pub selected: Option<u8>,
    pub candidate: HashSet<u8>,
    pub states: CellStates,
}

impl Cell {
    pub fn new() -> Self {
        Self {
            selected: None,
            candidate: HashSet::new(),
            states: CellStates::NONE,
        }
    }
}

pub struct Board {
    numbers: Vec<Cell>,
    callback_ptr: *mut c_void,
    update_callback: Option<extern fn(*mut c_void, u32, u32)>,
}

impl Board {
    pub fn empty() -> Self {
        Self {
            numbers: (0..81).map(|_| Cell::new()).collect(),
            callback_ptr: null_mut(),
            update_callback: None,
        }
    }

    pub fn set_update_callback(&mut self, cb: extern fn(*mut c_void, u32, u32)) {
        self.update_callback = Some(cb);
    }

    pub fn set_callback_ptr(&mut self, ptr: *mut c_void) {
        self.callback_ptr = ptr;
    }

    fn emit_update(&self, row: u32, column: u32) {
        if let Some(cb) = self.update_callback {
            cb(self.callback_ptr, row, column);
        }
    }

    fn emit_update_all(&self) {
        for i in 0..9 {
            for j in 0..9 {
                self.emit_update(i, j);
            }
        }
    }

    pub extern fn generate(&mut self) {
        // step 1. generate correct result
        self.initialize();
        while !self.can_resolve() {
            self.initialize();
        }

        // step 2. randomize
        self.randomize();

        // step 3. remove some block & ensure can be resolve
        let backup = self.numbers.clone();
        self.random_remove(55);

        // step 4. fill candidate & cleanup
        self.reset_candidate();
        self.init_selected_cells();

        // step 5. emit update all
        self.emit_update_all();
    }

    fn random_remove(&mut self, count: u32) {
        let mut removed = 0;
        let mut rng = rand::thread_rng();
        while removed != count {
            let pick = rng.gen_range(0, 81);
            if self.numbers[pick].selected.is_some() {
                self.numbers[pick].selected = None;
                removed += 1;
            }
        }
    }

    fn randomize(&mut self) {
        let mut rng = rand::thread_rng();
        for _ in 0..rng.gen_range(18, 36) {
            let row_or_column = rng.gen_bool(0.5);
            let block = rng.gen_range(0, 3);
            let pick1 = rng.gen_range(0, 3);
            let pick2 = rng.gen_range(0, 3);
            if pick1 == pick2 {
                continue;
            }

            if row_or_column {
                // switch row
                let row1 = block * 3 + pick1;
                let row2 = block * 3 + pick2;
                for i in 0..9 {
                    self.numbers.swap(row1 * 9 + i, row2 * 9 + i);
                }
            } else {
                // switch column
                let column1 = block * 3 + pick1;
                let column2 = block * 3 + pick2;
                for i in 0..9 {
                    self.numbers.swap(i * 9 + column1, i * 9 + column2);
                }
            }
        }
    }

    #[export_name = "sudoku_initialize"]
    pub extern fn initialize(&mut self) {
        // reset
        for x in &mut self.numbers {
            x.selected = None;
        }

        let mut rng = rand::thread_rng();
        let mut generated = 0;
        while generated != 11 {
            let pos = rng.gen_range(0, 81);
            if self.numbers[pos].selected.is_some() {
                continue;
            }

            self.numbers[pos].selected = Some(rng.gen_range(1, 10));
            if !self.check(pos / 9, pos % 9) {
                self.numbers[pos].selected = None;
                continue;
            }

            generated += 1;
        }
    }

    // remove candidate in pos
    fn remove_candidate(&mut self, row: usize, column: usize, candidate: u8) {
        debug_assert!(row < 9 && column < 9);

        let block_row = row / 3;
        let block_column = column / 3;

        for idx in 0..9 {
            // row
            self.numbers[row * 9 + idx].candidate.remove(&candidate);
            // column
            self.numbers[idx * 9 + column].candidate.remove(&candidate);
            // block
            let new_row = block_row * 3 + idx / 3;
            let new_column = block_column * 3 + idx % 3;
            self.numbers[new_row * 9 + new_column].candidate.remove(&candidate);
        }
    }

    // remove all selected candidate items
    fn init_selected_cells(&mut self) {
        let selected: Vec<(usize, u8)> = self.numbers.iter().enumerate()
            .filter(|(idx, cell)| cell.selected.is_some())
            .map(|(idx, cell)| (idx, cell.selected.unwrap()))
            .collect();

        for (idx, select) in selected {
            self.remove_candidate(idx / 9, idx % 9, select);
        }

        for cell in self.numbers.iter_mut() {
            if cell.selected.is_some() {
                cell.candidate.clear();
                cell.states |= CellStates::PRE_FILLED;
            }
        }
    }

    fn reset_candidate(&mut self) {
        for cell in self.numbers.iter_mut() {
            for i in 1..10 {
                cell.candidate.insert(i);
            }
        }
    }

    fn can_resolve2(&mut self) -> bool {
        self.init_selected_cells();

        true
    }

    fn next_unique_candidate(&self) -> Option<(usize, u8)> {
        self.numbers.iter().enumerate()
            .find(|(_, cell)| cell.candidate.len() == 1)
            .map(|(idx, cell)| (idx, *cell.candidate.iter().next().unwrap()))
    }

    fn can_resolve(&mut self) -> bool {
        let filled: Vec<usize> = self.numbers.iter()
            .enumerate().filter(|(_, cell)| {
            cell.selected.is_some()
        }).map(|(idx, _)| idx).collect();

        let mut current: isize = 0;
        let mut rollback = false;
        'fill: while current != 81 && current >= 0 {
            let curr = current as usize;
            if filled.contains(&curr) {
                if rollback {
                    current -= 1;
                } else {
                    current += 1;
                }
                continue;
            }

            rollback = false;

            let next_try = self.numbers[curr].selected.unwrap_or(0) + 1;

            // try number
            for i in next_try..10 {
                self.numbers[curr].selected = Some(i);
                if self.check(curr / 9, curr % 9) {
                    current += 1;
                    continue 'fill;
                }
            }

            // all failed, rollback
            self.numbers[curr].selected = None;
            rollback = true;
            current -= 1;
        }

        true
    }

    pub fn check(&self, row: usize, column: usize) -> bool {
        let x = self.cell(row, column);
        if x.selected.is_none() {
            return true;
        }

        // check row
        if self.row(row).iter()
            .filter(|y| y.selected == x.selected)
            .count() > 1 {
            return false;
        }

        // check column
        if self.column(column).iter()
            .filter(|y| y.selected == x.selected)
            .count() > 1 {
            return false;
        }

        // check block
        if self.block(row, column).iter()
            .filter(|y| y.selected == x.selected)
            .count() > 1 {
            return false;
        }

        true
    }

    pub fn set(&mut self, row: usize, column: usize, val: u8) {
        self.numbers[row * 9 + column].selected = Some(val);
    }

    fn row(&self, row: usize) -> Vec<&Cell> {
        debug_assert!(row < 9);

        (0..9).map(|x|
            &self.numbers[row * 9 + x])
            .collect()
    }

    fn column(&self, column: usize) -> Vec<&Cell> {
        debug_assert!(column < 9);

        (0..9).map(|x|
            &self.numbers[column + x * 9])
            .collect()
    }

    pub fn block(&self, row: usize, column: usize) -> Vec<&Cell> {
        let mut r = vec![];
        let br = row / 3;
        let bc = column / 3;
        for idx in 0..9 {
            let new_row = br * 3 + idx / 3;
            let new_column = bc * 3 + idx % 3;
            r.push(&self.numbers[new_row * 9 + new_column]);
        }

        r
    }

    pub fn cell(&self, row: usize, column: usize) -> &Cell {
        debug_assert!(row < 9 && column < 9);

        &self.numbers[row * 9 + column]
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for row in 0..9 {
            if row % 3 == 0 {
                writeln!(f, "+-----+-----+-----+")?;
            }
            write!(f, "|")?;
            for column in 0..9 {
                let index = row * 9 + column;
                let s = self.numbers[index].selected
                    .map(|x| x.to_string())
                    .unwrap_or(" ".to_string());
                write!(f, "{}", s)?;
                if column % 3 == 2 {
                    write!(f, "|")?;
                } else {
                    write!(f, " ")?;
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "+-----+-----+-----+")
    }
}

#[cfg(test)]
mod tests {
    use crate::board::Board;

    #[test]
    fn test_remove_candidate()
    {
        let mut board = Board::empty();
        board.reset_candidate();
        assert!(!board.cell(0, 0).candidate.contains(&0));
        assert!(board.cell(0, 0).candidate.contains(&1));
        board.remove_candidate(0, 0, 1);
        assert!(!board.cell(0, 0).candidate.contains(&1));
        assert!(!board.cell(0, 8).candidate.contains(&1));
        assert!(!board.cell(1, 0).candidate.contains(&1));
        assert!(!board.cell(2, 0).candidate.contains(&1));
        assert!(!board.cell(1, 1).candidate.contains(&1));
        assert!(!board.cell(1, 2).candidate.contains(&1));
        assert!(!board.cell(2, 2).candidate.contains(&1));
        assert!(board.cell(3, 3).candidate.contains(&1));
        assert!(board.cell(3, 1).candidate.contains(&1));
        assert!(board.cell(1, 3).candidate.contains(&1));
        assert!(board.cell(1, 8).candidate.contains(&1));
        assert!(board.cell(8, 1).candidate.contains(&1));

        board.remove_candidate(4, 8, 4);
        assert!(!board.cell(4, 0).candidate.contains(&4));
    }

    #[test]
    fn test_get_block()
    {
        let mut board = Board::empty();
        board.set(3, 3, 1);
        board.set(3, 4, 2);
        board.set(3, 5, 3);
        board.set(4, 3, 4);
        board.set(4, 4, 5);
        board.set(4, 5, 6);
        board.set(5, 3, 7);
        board.set(5, 4, 8);
        board.set(5, 5, 9);

        let block = board.block(4, 4);
        assert_eq!(block[0].selected, Some(1));
        assert_eq!(block[1].selected, Some(2));
        assert_eq!(block[4].selected, Some(5));
        assert_eq!(block[5].selected, Some(6));
        assert_eq!(block[7].selected, Some(8));
        assert_eq!(block[8].selected, Some(9));
    }

    #[test]
    fn test_randomize()
    {
        let mut board = Board::empty();
        board.initialize();
        while !board.can_resolve() {
            board.initialize();
        }

        board.randomize();
        assert!(board.can_resolve());
    }

    #[test]
    fn test_last_block()
    {
        let mut board = Board::empty();
        board.set(6, 6, 1);
        board.set(6, 7, 2);
        board.set(6, 8, 3);
        board.set(7, 6, 4);
        board.set(7, 7, 5);
        board.set(7, 8, 6);
        board.set(8, 6, 7);
        board.set(8, 7, 8);
        board.set(8, 8, 9);

        let block = board.block(8, 8);
        assert_eq!(block[0].selected, Some(1));
        assert_eq!(block[1].selected, Some(2));
        assert_eq!(block[4].selected, Some(5));
        assert_eq!(block[5].selected, Some(6));
        assert_eq!(block[7].selected, Some(8));
        assert_eq!(block[8].selected, Some(9));
    }
}
