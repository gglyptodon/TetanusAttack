use bevy::prelude::Resource;
use rand::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockColor {
    Red,
    Green,
    Blue,
    Yellow,
    Purple,
}

#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub color: BlockColor,
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

impl Cursor {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub fn move_by(&mut self, dx: isize, dy: isize, width: usize, height: usize) -> bool {
        if width < 2 || height == 0 {
            return false;
        }
        let max_x = width - 2;
        let max_y = height - 1;
        let nx = (self.x as isize + dx).clamp(0, max_x as isize) as usize;
        let ny = (self.y as isize + dy).clamp(0, max_y as isize) as usize;
        let changed = nx != self.x || ny != self.y;
        self.x = nx;
        self.y = ny;
        changed
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SwapCmd {
    pub ax: usize,
    pub ay: usize,
    pub bx: usize,
    pub by: usize,
}

impl SwapCmd {
    pub fn right_of(x: usize, y: usize) -> Self {
        Self {
            ax: x,
            ay: y,
            bx: x + 1,
            by: y,
        }
    }
}

#[derive(Resource)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    cells: Vec<Option<Block>>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![None; width * height],
        }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<Block> {
        self.cells[self.idx(x, y)]
    }

    pub fn set(&mut self, x: usize, y: usize, block: Option<Block>) {
        let idx = self.idx(x, y);
        self.cells[idx] = block;
    }

    pub fn swap(&mut self, ax: usize, ay: usize, bx: usize, by: usize) {
        let a = self.idx(ax, ay);
        let b = self.idx(bx, by);
        self.cells.swap(a, b);
    }

    pub fn swap_in_bounds(&mut self, cmd: SwapCmd) -> bool {
        if cmd.ax >= self.width
            || cmd.bx >= self.width
            || cmd.ay >= self.height
            || cmd.by >= self.height
        {
            return false;
        }
        self.swap(cmd.ax, cmd.ay, cmd.bx, cmd.by);
        true
    }

    pub fn fill_test_pattern(&mut self) {
        let colors = [
            BlockColor::Red,
            BlockColor::Green,
            BlockColor::Blue,
            BlockColor::Yellow,
            BlockColor::Purple,
        ];
        let filled_rows = self.height / 2;
        for y in 0..filled_rows {
            for x in 0..self.width {
                let c = colors[(x + y) % colors.len()];
                self.set(x, y, Some(Block { color: c }));
            }
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(None);
    }

    pub fn resolve(&mut self) -> u32 {
        let mut total_cleared = 0;
        let mut passes = 0;
        loop {
            let marks = self.find_matches();
            if marks.iter().all(|m| !*m) {
                break;
            }
            total_cleared += self.clear_matches(&marks);
            self.apply_gravity();
            passes += 1;
            if passes > 10 {
                break;
            }
        }
        total_cleared
    }

    pub fn apply_gravity(&mut self) {
        for x in 0..self.width {
            let mut write_y = 0;
            for y in 0..self.height {
                let idx = self.idx(x, y);
                if let Some(block) = self.cells[idx] {
                    if y != write_y {
                        let write_idx = self.idx(x, write_y);
                        self.cells[write_idx] = Some(block);
                        self.cells[idx] = None;
                    }
                    write_y += 1;
                }
            }
        }
    }

    pub fn apply_gravity_step(&mut self) -> bool {
        let mut moved = false;
        if self.height < 2 {
            return false;
        }
        for x in 0..self.width {
            for y in 1..self.height {
                let idx = self.idx(x, y);
                let below = self.idx(x, y - 1);
                if self.cells[idx].is_some() && self.cells[below].is_none() {
                    self.cells[below] = self.cells[idx];
                    self.cells[idx] = None;
                    moved = true;
                }
            }
        }
        moved
    }

    fn find_matches(&self) -> Vec<bool> {
        let mut marks = vec![false; self.width * self.height];

        for y in 0..self.height {
            let mut run_start = 0;
            let mut run_len = 1;
            for x in 1..self.width {
                if self.same_color(x, y, x - 1, y) {
                    run_len += 1;
                } else {
                    if run_len >= 3 {
                        for rx in run_start..run_start + run_len {
                            marks[self.idx(rx, y)] = true;
                        }
                    }
                    run_start = x;
                    run_len = 1;
                }
            }
            if run_len >= 3 {
                for rx in run_start..run_start + run_len {
                    marks[self.idx(rx, y)] = true;
                }
            }
        }

        for x in 0..self.width {
            let mut run_start = 0;
            let mut run_len = 1;
            for y in 1..self.height {
                if self.same_color(x, y, x, y - 1) {
                    run_len += 1;
                } else {
                    if run_len >= 3 {
                        for ry in run_start..run_start + run_len {
                            marks[self.idx(x, ry)] = true;
                        }
                    }
                    run_start = y;
                    run_len = 1;
                }
            }
            if run_len >= 3 {
                for ry in run_start..run_start + run_len {
                    marks[self.idx(x, ry)] = true;
                }
            }
        }

        marks
    }

    fn clear_matches(&mut self, marks: &[bool]) -> u32 {
        let mut cleared = 0;
        for i in 0..self.cells.len() {
            if marks[i] {
                self.cells[i] = None;
                cleared += 1;
            }
        }
        cleared
    }

    fn same_color(&self, ax: usize, ay: usize, bx: usize, by: usize) -> bool {
        match (self.get(ax, ay), self.get(bx, by)) {
            (Some(a), Some(b)) => a.color == b.color,
            _ => false,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    pub fn push_bottom_row(&mut self) {
        if self.height == 0 || self.width == 0 {
            return;
        }
        if self.top_row_occupied() {
            return;
        }
        for y in (1..self.height).rev() {
            for x in 0..self.width {
                let below = self.idx(x, y - 1);
                let here = self.idx(x, y);
                self.cells[here] = self.cells[below];
            }
        }

        let mut rng = thread_rng();
        for x in 0..self.width {
            let idx = self.idx(x, 0);
            let mut color = random_color(&mut rng);
            for _ in 0..10 {
                if !self.would_create_match(x, 0, color) {
                    break;
                }
                color = random_color(&mut rng);
            }
            self.cells[idx] = Some(Block { color });
        }
    }

    pub fn top_row_occupied(&self) -> bool {
        if self.height == 0 {
            return false;
        }
        let y = self.height - 1;
        for x in 0..self.width {
            if self.get(x, y).is_some() {
                return true;
            }
        }
        false
    }

    fn would_create_match(&self, x: usize, y: usize, color: BlockColor) -> bool {
        let left1 = if x >= 1 { self.get(x - 1, y) } else { None };
        let left2 = if x >= 2 { self.get(x - 2, y) } else { None };
        let right1 = if x + 1 < self.width {
            self.get(x + 1, y)
        } else {
            None
        };
        let right2 = if x + 2 < self.width {
            self.get(x + 2, y)
        } else {
            None
        };

        let horiz_left = left1.map(|b| b.color == color).unwrap_or(false)
            && left2.map(|b| b.color == color).unwrap_or(false);
        let horiz_right = right1.map(|b| b.color == color).unwrap_or(false)
            && right2.map(|b| b.color == color).unwrap_or(false);
        let horiz_split = left1.map(|b| b.color == color).unwrap_or(false)
            && right1.map(|b| b.color == color).unwrap_or(false);

        if horiz_left || horiz_right || horiz_split {
            return true;
        }

        if y + 2 < self.height {
            let up1 = self.get(x, y + 1).map(|b| b.color == color).unwrap_or(false);
            let up2 = self.get(x, y + 2).map(|b| b.color == color).unwrap_or(false);
            if up1 && up2 {
                return true;
            }
        }

        false
    }
}

fn random_color(rng: &mut ThreadRng) -> BlockColor {
    match rng.gen_range(0..5) {
        0 => BlockColor::Red,
        1 => BlockColor::Green,
        2 => BlockColor::Blue,
        3 => BlockColor::Yellow,
        _ => BlockColor::Purple,
    }
}
