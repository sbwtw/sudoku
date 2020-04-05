use rand;

use std::fmt;
use std::os::raw::c_void;
use std::ptr::null_mut;
use rand::Rng;

use crate::cell::*;

pub struct Board {
    numbers: Vec<Cell>,
    current_highlight: Option<u8>,
    callback_ptr: *mut c_void,
    update_callback: Option<extern fn(*mut c_void, u32, u32)>,
}

impl Board {
    pub fn empty() -> Self {
        Self {
            numbers: (0..81).map(|_| Cell::new()).collect(),
            current_highlight: None,
            callback_ptr: null_mut(),
            update_callback: None,
        }
    }

    pub fn set_current_highlight(&mut self, high_light: Option<u8>) {
        if self.current_highlight == high_light {
            return;
        }

        let mut updates = vec![];
        for (idx, cell) in self.numbers.iter_mut().enumerate() {
            if self.current_highlight.is_some() {
                if self.current_highlight == cell.selected() {
                    cell.set_states(cell.states() & !CellStates::HIGH_LIGHT);
                    updates.push(idx);
                }
            }

            if high_light.is_some() {
                if high_light == cell.selected() {
                    cell.set_states(cell.states() | CellStates::HIGH_LIGHT);
                    updates.push(idx);
                }
            }
        }

        for i in updates {
            self.emit_update_cell(i / 9, i % 9);
        }

        self.current_highlight = high_light;
    }

    pub fn set_update_callback(&mut self, cb: extern fn(*mut c_void, u32, u32)) {
        self.update_callback = Some(cb);
    }

    pub fn set_callback_ptr(&mut self, ptr: *mut c_void) {
        self.callback_ptr = ptr;
    }

    fn emit_update_effect_cell(&self, row: usize, column: usize) {
        if let Some(cb) = self.update_callback {
            for idx in self.effect_cell_indexes(row, column).iter() {
                cb(self.callback_ptr, *idx as u32 / 9, *idx as u32 % 9);
            }
        }
    }

    fn emit_update_cell(&self, row: usize, column: usize) {
        if let Some(cb) = self.update_callback {
            cb(self.callback_ptr, row as u32, column as u32);
        }
    }

    fn emit_update_all(&self) {
        if let Some(cb) = self.update_callback {
            for i in 0..9 {
                for j in 0..9 {
                    cb(self.callback_ptr, i as u32, j as u32);
                }
            }
        }
    }

    pub fn generate(&mut self) {
        // step 1. generate correct result
        self.initialize();
        //self.try_resolve();
        while !self.try_resolve() {
            self.initialize();
            println!("Can't resolve, generate new board: \n{}", *self);
        }
        println!("Initialized:\n{}", *self);

        // step 2. randomize
        let mut rng = rand::thread_rng();
        let pass_count = rng.gen_range(18, 36);
        self.randomize(pass_count);
        println!("Randomized:\n{}", *self);

        // step 3. remove some block & ensure can be resolve
        // let backup = self.numbers.clone();
        self.random_remove(50);
        println!("Blocks Removed:\n{}", *self);

        // step 4. fill candidate & cleanup
        self.reset_init_state();

        // step 5. emit update all
        self.emit_update_all();
    }

    fn random_remove(&mut self, count: u32) {
        let mut removed = 0;
        let mut rng = rand::thread_rng();
        while removed != count {
            let pick = rng.gen_range(0, 81);
            if self.numbers[pick].selected().is_some() {
                self.numbers[pick].set_select(None);
                removed += 1;
            }
        }
    }

    // 在保持解不变的情况化进行随机化处理
    fn randomize(&mut self, pass_count: usize) {
        let mut rng = rand::thread_rng();
        for _ in 0..pass_count {
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
            x.set_select(None);
            x.set_states(CellStates::NONE);
        }

        let mut rng = rand::thread_rng();
        let mut generated = 0;
        while generated != 11 {
            let pos = rng.gen_range(0, 81);
            let cell = self.cell(pos / 9, pos % 9).clone();
            if cell.selected().is_some() {
                continue;
            }

            let candidate = rng.gen_range(1, 10);
            if cell.has_candidate(candidate) {
                self.set(pos / 9, pos % 9, Some(candidate));
                self.cell_mut(pos / 9, pos % 9).set_states(cell.states() | CellStates::PRE_FILLED);
                generated += 1;
            }
        }
    }

    fn reset_init_state(&mut self) {
        // collect all selected
        let selected: Vec<(usize, u8)> = self.numbers.iter().enumerate()
            .filter(|(_, cell)| cell.selected().is_some())
            .map(|(idx, cell)| (idx, cell.selected().unwrap()))
            .collect();

        // re-generate board
        self.numbers = (0..81).map(|_| Cell::new()).collect();

        // write new data
        for (idx, select) in selected {
            let cell = self.cell_mut(idx / 9, idx % 9);
            cell.set_states(cell.states() | CellStates::PRE_FILLED);

            self.set(idx / 9, idx % 9, Some(select));
        }
    }

    fn try_resolve(&mut self) -> bool {
        let filled: Vec<isize> = self.numbers.iter()
            .enumerate().filter(|(_, cell)| {
            cell.selected().is_some()
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

            if try_times > 10000 { return false; }
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
        if x.selected().is_none() {
            return true;
        }

        for idx in self.effect_cell_indexes(row, column).iter() {
            if self.numbers[*idx].selected() == x.selected() {
                return false;
            }
        }

        true
    }

    pub fn set(&mut self, row: usize, column: usize, val: Option<u8>) -> bool {
        // add candidate back if already has value
        if let Some(v) = self.cell(row, column).selected() {
            self.cell_mut(row, column).add_candidate(v);

            // add effect candidates
            for idx in self.effect_cell_indexes(row, column).iter() {
                self.numbers[*idx].add_candidate(v);
            }
        }

        // current cell updates
        let highlight = val.is_some() && self.current_highlight == val;
        let cell = self.cell_mut(row, column);
        cell.set_select(val);

        // update highlight status
        if highlight {
            cell.set_states(cell.states() | CellStates::HIGH_LIGHT);
        } else {
            cell.set_states(cell.states() & !CellStates::HIGH_LIGHT);
        }

        // set new value
        let mut error_occured = false;
        if let Some(v) = val {
            let cell = self.cell_mut(row, column);
            cell.set_states(cell.states() | CellStates::FILLED);
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
            let cell = self.cell_mut(row, column);
            cell.set_states(cell.states() & !CellStates::FILLED);
        }

        // emit updates
        self.emit_update_effect_cell(row, column);
        self.emit_update_cell(row, column);

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
                let s = self.numbers[index].selected()
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
    use crate::board::CellStates;

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

        board.randomize(3);
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

    #[test]
    fn test_cell_state() {
        let mut board = Board::empty();
        board.initialize();

        let mut pre_filled = 0;
        for i in 0..=80 {
            if board.numbers[i].selected().is_some() {
                pre_filled += 1;
                assert!(board.numbers[i].states() == CellStates::PRE_FILLED);
            }
        }
        assert_eq!(pre_filled, 11);

        let mut board = Board::empty();
        board.initialize();
        while !board.try_resolve() {
            board.initialize();
        }

        let mut pre_filled = 0;
        let mut filled = 0;
        for i in 0..=80 {
            if board.cell(i / 9, i % 9).states() == CellStates::PRE_FILLED {
                pre_filled += 1;
            }
            if board.cell(i / 9, i % 9).states() == CellStates::FILLED {
                filled += 1;
            }
        }
        assert_eq!(11, pre_filled);
        assert_eq!(70, filled);
    }

    #[test]
    fn test_highlight_set() {
        let mut board = Board::empty();
        assert_eq!(false, (board.cell(0, 0).states() & CellStates::HIGH_LIGHT) == CellStates::HIGH_LIGHT);

        board.set_current_highlight(Some(1));
        board.set(0, 0, Some(1));
        assert_eq!(true, (board.cell(0, 0).states() & CellStates::HIGH_LIGHT) == CellStates::HIGH_LIGHT);

        board.set(0, 0, None);
        assert_eq!(false, (board.cell(0, 0).states() & CellStates::HIGH_LIGHT) == CellStates::HIGH_LIGHT);

        board.set_current_highlight(None);
        assert_eq!(false, (board.cell(0, 0).states() & CellStates::HIGH_LIGHT) == CellStates::HIGH_LIGHT);

        board.set(0, 0, Some(2));
        board.set_current_highlight(Some(2));
        assert_eq!(true, (board.cell(0, 0).states() & CellStates::HIGH_LIGHT) == CellStates::HIGH_LIGHT);

        board.set_current_highlight(None);
        assert_eq!(false, (board.cell(0, 0).states() & CellStates::HIGH_LIGHT) == CellStates::HIGH_LIGHT);
    }
}
