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

    pub fn set_states(&mut self, states: CellStates) {
        self.states = states;

        if (states & CellStates::PRE_FILLED) == CellStates::PRE_FILLED {
            self.candidate = [0i8; 10];
        }
    }

    pub fn selected(&self) -> Option<u8> {
        self.selected
    }

    pub fn set_select(&mut self, val: Option<u8>) {
        self.selected = val;
    }

    pub fn has_candidate(&self, candidate: u8) -> bool {
        debug_assert!(candidate > 0 && candidate <= 9);

        self.candidate[candidate as usize] > 0
    }

    pub fn best_candidates(&self) -> Vec<u8> {
        let mut r = vec![];
        let low = self.selected.unwrap_or(0) as usize + 1;

        for i in low..=9 {
            if self.candidate[i] > 0 {
                r.push(i as u8);
            }
        }

        r
    }

    pub fn add_candidate(&mut self, candidate: u8) {
        if !self.is_prefilled() {
            self.candidate[candidate as usize] += 1;
            debug_assert!(self.candidate[candidate as usize] <= 1);
        }
    }

    // return true if error occured
    pub fn remove_candidate(&mut self, candidate: u8) -> bool {
        if self.is_prefilled() {
            return false;
        }

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

    pub fn reset_candidate(&mut self) {
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

    pub fn is_prefilled(&self) -> bool {
        (self.states & CellStates::PRE_FILLED) == CellStates::PRE_FILLED
    }
}

