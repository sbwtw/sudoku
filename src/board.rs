
use rand;

use std::fmt;
use std::os::raw::c_void;
use std::ptr::null_mut;
use rand::Rng;
use bitflags::bitflags;

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
    selected: Option<u8>,
    candidate: [i8; 10],
    states: CellStates,
}

impl Cell {
    pub fn new() -> Self {
        Self {
            selected: None,
            candidate: [1i8; 10],
            states: CellStates::NONE,
        }
    }

    pub fn states(&self) -> CellStates {
        self.states
    }

    pub fn selected(&self) -> Option<u8> {
        self.selected
    }

    fn has_candidate(&self, candidate: u8) -> bool {
        debug_assert!(candidate > 0 && candidate <= 9);

        self.candidate[candidate as usize] > 0
    }

    fn best_candidates(&self) -> Vec<u8> {
        let mut r = vec![];
        let low = self.selected.unwrap_or(0) as usize + 1;

        for i in low..=9 {
            if self.candidate[i] > 0 {
                r.push(i as u8);
            }
        }

        r
    }

    fn add_candidate(&mut self, candidate: u8) {
        self.candidate[candidate as usize] += 1;
    }

    // return true if error occured
    fn remove_candidate(&mut self, candidate: u8) -> bool {
        self.candidate[candidate as usize] -= 1;

        if self.selected.is_some() {
            return false;
        }

        for i in 1..=9 {
            if self.candidate[i] > 0 {
                return false;
            }
        }

        true
    }

    fn reset_candidate(&mut self) {
        self.candidate = [1i8; 10];
    }

    pub fn candidate_u32(&self) -> u32 {
        let mut flags: u32 = 0;

        for i in 1..=9 {
            if self.candidate[i] > 0 {
                flags |= 1 << i;
            }
        }

        flags
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

    fn emit_update_cell(&self, row: u32, column: u32) {
        if let Some(cb) = self.update_callback {
            cb(self.callback_ptr, row, column);
        }
    }

    fn emit_update_index(&self, index: usize) {
        if let Some(cb) = self.update_callback {
            cb(self.callback_ptr, (index / 9) as u32, (index % 9) as u32);
        }
    }

    fn emit_update_all(&self) {
        for i in 0..9 {
            for j in 0..9 {
                self.emit_update_cell(i, j);
            }
        }
    }

    pub fn generate(&mut self) {
        // step 1. generate correct result
        self.initialize();
        println!("{}", *self);
        self.try_resolve();
        //while !self.try_resolve() {
            //self.initialize();
            //println!("Can't resolve, generate new board: \n{}", *self);
        //}

        // step 2. randomize
        self.randomize();

        // step 3. remove some block & ensure can be resolve
        //let backup = self.numbers.clone();
        //self.random_remove(10);

        // step 4. fill candidate & cleanup
        //self.reset_candidate();
        //self.init_selected_cells();

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

    pub fn initialize(&mut self) {
        // reset
        for x in &mut self.numbers {
            x.reset_candidate();
            x.selected = None;
            x.states = CellStates::NONE;
        }

        let mut rng = rand::thread_rng();
        let mut generated = 0;
        while generated != 11 {
            let pos = rng.gen_range(0, 81);
            if self.numbers[pos].selected.is_some() {
                continue;
            }

            let candidate = rng.gen_range(1, 10);
            if self.numbers[pos].has_candidate(candidate) {
                self.set(pos / 9, pos % 9, Some(candidate));
                self.numbers[pos].states |= CellStates::PRE_FILLED;
                generated += 1;
            }
        }
    }

    // remove all selected candidate items
    fn init_selected_cells(&mut self) {
        let selected: Vec<(usize, u8)> = self.numbers.iter().enumerate()
            .filter(|(_, cell)| cell.selected.is_some())
            .map(|(idx, cell)| (idx, cell.selected.unwrap()))
            .collect();

        for (idx, select) in selected {
            self.cell_mut(idx / 9, idx % 9).remove_candidate(select);
        }

        for cell in self.numbers.iter_mut() {
            if cell.selected.is_some() {
                cell.reset_candidate();
                cell.states |= CellStates::PRE_FILLED;
            }
        }
    }

    fn reset_candidate(&mut self) {
        for cell in self.numbers.iter_mut() {
            cell.reset_candidate();
        }
    }

    fn try_resolve(&mut self) -> bool {
        let filled: Vec<isize> = self.numbers.iter()
            .enumerate().filter(|(_, cell)| {
            cell.selected.is_some()
        }).map(|(idx, _)| idx as isize).collect();

        let mut current: isize = 0;
        let mut rollback = false;
        let mut try_times = 0;
        'fill: while current != 81 && current >= 0 {
            if filled.contains(&current) {
                if rollback {
                    current -= 1;
                } else {
                    current += 1;
                }
                continue 'fill;
            }

            if try_times > 100000 { return false; }
            try_times += 1;

            rollback = false;
            let cell = &self.numbers[current as usize];
            let available_candidates = cell.best_candidates();

            //println!("try to fill cell {} with candidates {:?}", current, available_candidates);
            for try_num in available_candidates.iter() {
                //println!("write {} to cell {}", try_num, current);
                if !self.set(current as usize / 9, current as usize % 9, Some(*try_num)) {
                    current += 1;
                    continue 'fill;
                }
            }

            //println!("rollback cell {}", current);
            // all failed, rollback
            self.set(current as usize / 9, current as usize % 9, None);
            rollback = true;
            current -= 1;
        }

        current == 81
    }

    pub fn check(&self, row: usize, column: usize) -> bool {
        let x = self.cell(row, column);
        if x.selected.is_none() {
            return true;
        }

        for idx in self.effect_cell_indexes(row, column).iter() {
            if self.numbers[*idx].selected == x.selected {
                return false;
            }
        }

        true
    }

    pub fn set(&mut self, row: usize, column: usize, val: Option<u8>) -> bool {
        if let Some(v) = self.cell(row, column).selected {
            self.cell_mut(row, column).add_candidate(v);

            // add effect candidates
            for idx in self.effect_cell_indexes(row, column).iter() {
                self.numbers[*idx].add_candidate(v);
            }
        }

        self.cell_mut(row, column).selected = val;

        let mut error_occured = false;
        if let Some(v) = val {
            let cell = self.cell_mut(row, column);
            cell.states |= CellStates::FILLED;
            if cell.remove_candidate(v) {
                //println!("No candidate on cell {} {}", row, column);
                error_occured = true;
            }

            // remove effect candidates
            for idx in self.effect_cell_indexes(row, column).iter() {
                if self.numbers[*idx].remove_candidate(v) {
                    error_occured = true;
                    //println!("No candidate on cell {} {}", row, column);
                }
            }
        } else {
            self.cell_mut(row, column).states &= !CellStates::FILLED;
        }

        error_occured
    }

    fn effect_cell_indexes(&self, row: usize, column: usize) -> [usize; 20] {
        let mut indexes = [0; 20];
        let mut index = 0;

        // same row
        for i in 0..=8 {
            if i != column {
                indexes[index] = row * 9 + i;
                index += 1;
            }
        }

        // same column
        for i in 0..=8 {
            if i != row {
                indexes[index] = i * 9 + column;
                index += 1;
            }
        }

        // same block
        let br = (row / 3) * 3;
        let bc = (column / 3) * 3;
        for r in 0..=2 {
            for c in 0..=2 {
                let cr = br + r;
                let cc = bc + c;


                if cr != row && cc != column {
                    indexes[index] = cr * 9 + cc;
                    index += 1;
                }
            }
        }

        debug_assert!(20 == index);
        indexes
    }

    pub fn cell(&self, row: usize, column: usize) -> &Cell {
        debug_assert!(row < 9 && column < 9);

        &self.numbers[row * 9 + column]
    }

    pub fn cell_mut(&mut self, row: usize, column: usize) -> &mut Cell {
        &mut self.numbers[row * 9 + column]
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
        assert!(board.cell(0, 0).has_candidate(1));
        assert!(board.cell(0, 0).has_candidate(5));
        assert!(board.cell(0, 0).has_candidate(9));

        board.set(0, 0, Some(1));
        assert!(!board.cell(0, 0).has_candidate(1));
        assert!(!board.cell(0, 8).has_candidate(1));
        assert!(!board.cell(8, 0).has_candidate(1));
        assert!(!board.cell(2, 2).has_candidate(1));
        assert!(board.cell(2, 2).has_candidate(2));

        board.set(1, 1, Some(2));
        assert!(!board.cell(2, 2).has_candidate(2));
        assert!(board.cell(4, 4).has_candidate(2));
    }

    #[test]
    fn test_check()
    {
        let mut board = Board::empty();
        board.set(2, 3, Some(1));
        assert!(board.check(2, 3));

        board.set(1, 5, Some(1));
        assert!(!board.check(2, 3));
        assert!(!board.check(1, 5));
    }

    #[test]
    fn test_randomize()
    {
        let mut board = Board::empty();
        board.initialize();
        while !board.try_resolve() {
            board.initialize();
        }

        board.randomize();
        assert!(board.try_resolve());
    }

    #[test]
    fn test_effect_cells()
    {
        let board = Board::empty();
        let indexes = board.effect_cell_indexes(6, 3);
        assert!(!indexes.contains(&0));
        assert!(!indexes.contains(&53));
        assert!(indexes.contains(&54));
        assert!(indexes.contains(&62));
        assert!(!indexes.contains(&63));
        assert!(!indexes.contains(&65));
        assert!(indexes.contains(&66));
        assert!(indexes.contains(&67));
        assert!(indexes.contains(&68));
        assert!(!indexes.contains(&69));
        assert!(indexes.contains(&77));
        assert!(!indexes.contains(&78));

        let indexes = board.effect_cell_indexes(0, 0);
        assert!(!indexes.contains(&0));
        assert!(indexes.contains(&1));
        assert!(indexes.contains(&2));
        assert!(indexes.contains(&3));
        assert!(indexes.contains(&4));
        assert!(indexes.contains(&5));
        assert!(indexes.contains(&6));
        assert!(indexes.contains(&7));
        assert!(indexes.contains(&8));

        assert!(indexes.contains(&9));
        assert!(indexes.contains(&18));
        assert!(indexes.contains(&27));
        assert!(indexes.contains(&36));
        assert!(indexes.contains(&45));
        assert!(indexes.contains(&54));
        assert!(indexes.contains(&63));
        assert!(indexes.contains(&72));

        assert!(indexes.contains(&10));
        assert!(indexes.contains(&11));
        assert!(indexes.contains(&19));
        assert!(indexes.contains(&20));

        let indexes = board.effect_cell_indexes(2, 3);
        assert!(indexes.contains(&3));
        assert!(indexes.contains(&23));
    }
}
