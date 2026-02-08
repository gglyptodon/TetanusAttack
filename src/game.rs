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
pub enum Block {
    Normal { color: BlockColor },
    Garbage { cracked: bool },
}

impl Block {
    pub fn color(self) -> Option<BlockColor> {
        match self {
            Block::Normal { color } => Some(color),
            Block::Garbage { .. } => None,
        }
    }

    pub fn is_garbage(self) -> bool {
        matches!(self, Block::Garbage { .. })
    }
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
        if self.get(cmd.ax, cmd.ay).map(|b| b.is_garbage()).unwrap_or(false)
            || self.get(cmd.bx, cmd.by).map(|b| b.is_garbage()).unwrap_or(false)
        {
            return false;
        }
        self.swap(cmd.ax, cmd.ay, cmd.bx, cmd.by);
        true
    }

    pub fn fill_test_pattern(&mut self) {
        let filled_rows = self.height / 2;
        let mut rng = thread_rng();
        for y in 0..filled_rows {
            for x in 0..self.width {
                let mut color = random_color(&mut rng);
                for _ in 0..10 {
                    if !self.would_create_match(x, y, color) {
                        break;
                    }
                    color = random_color(&mut rng);
                }
                self.set(x, y, Some(Block::Normal { color }));
            }
        }
    }

    pub fn clear(&mut self) {
        self.cells.fill(None);
    }

    pub fn clear_matches_once(&mut self) -> u32 {
        self.clear_matches_once_with_stats().cleared
    }

    pub fn clear_matches_once_with_stats(&mut self) -> ClearStats {
        let marks = self.find_matches();
        if marks.iter().all(|m| !*m) {
            return ClearStats {
                cleared: 0,
                groups: 0,
                marks,
            };
        }
        let groups = self.count_match_groups(&marks);
        let cleared = self.clear_matches(&marks);
        ClearStats {
            cleared,
            groups,
            marks,
        }
    }

    pub fn has_matches(&self) -> bool {
        let marks = self.find_matches();
        marks.iter().any(|m| *m)
    }

    pub fn apply_gravity(&mut self) {
        while self.apply_gravity_step() {}
    }

    pub fn apply_gravity_step(&mut self) -> bool {
        let mut moved = false;
        if self.height < 2 {
            return false;
        }
        let snapshot = self.cells.clone();
        let mut normal_moves: Vec<(usize, usize, Block)> = Vec::new();
        for x in 0..self.width {
            for y in 1..self.height {
                let idx = self.idx(x, y);
                let below = self.idx(x, y - 1);
                if let Some(Block::Normal { .. }) = snapshot[idx] {
                    if snapshot[below].is_none() {
                        normal_moves.push((idx, below, snapshot[idx].unwrap()));
                    }
                }
            }
        }

        let mut visited = vec![false; snapshot.len()];
        let mut garbage_moves: Vec<(usize, usize, Block)> = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if visited[idx] {
                    continue;
                }
                if let Some(Block::Garbage { .. }) = snapshot[idx] {
                    let mut stack = vec![(x, y)];
                    let mut component: Vec<(usize, usize)> = Vec::new();
                    visited[idx] = true;
                    while let Some((cx, cy)) = stack.pop() {
                        component.push((cx, cy));
                        let neighbors = [
                            (cx.wrapping_sub(1), cy, cx > 0),
                            (cx + 1, cy, cx + 1 < self.width),
                            (cx, cy.wrapping_sub(1), cy > 0),
                            (cx, cy + 1, cy + 1 < self.height),
                        ];
                        for (nx, ny, ok) in neighbors {
                            if !ok {
                                continue;
                            }
                            let nidx = self.idx(nx, ny);
                            if !visited[nidx] {
                                if let Some(Block::Garbage { .. }) = snapshot[nidx] {
                                    visited[nidx] = true;
                                    stack.push((nx, ny));
                                }
                            }
                        }
                    }

                    let mut in_component = vec![false; snapshot.len()];
                    for &(cx, cy) in &component {
                        in_component[self.idx(cx, cy)] = true;
                    }
                    let mut can_fall = true;
                    for &(cx, cy) in &component {
                        if cy == 0 {
                            can_fall = false;
                            break;
                        }
                        let below = self.idx(cx, cy - 1);
                        if snapshot[below].is_some() && !in_component[below] {
                            can_fall = false;
                            break;
                        }
                    }

                    if can_fall {
                        for (cx, cy) in component {
                            let from = self.idx(cx, cy);
                            let to = self.idx(cx, cy - 1);
                            garbage_moves.push((from, to, snapshot[from].unwrap()));
                        }
                    }
                }
            }
        }

        if !normal_moves.is_empty() || !garbage_moves.is_empty() {
            moved = true;
            for (from, _, _) in normal_moves.iter().chain(garbage_moves.iter()) {
                self.cells[*from] = None;
            }
            for (_, to, block) in normal_moves.into_iter().chain(garbage_moves.into_iter()) {
                self.cells[to] = Some(block);
            }
        }
        moved
    }

    pub fn has_falling_garbage(&self) -> bool {
        let mut visited = vec![false; self.cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if visited[idx] {
                    continue;
                }
                if let Some(Block::Garbage { .. }) = self.cells[idx] {
                    let mut stack = vec![(x, y)];
                    let mut component: Vec<(usize, usize)> = Vec::new();
                    visited[idx] = true;
                    while let Some((cx, cy)) = stack.pop() {
                        component.push((cx, cy));
                        let neighbors = [
                            (cx.wrapping_sub(1), cy, cx > 0),
                            (cx + 1, cy, cx + 1 < self.width),
                            (cx, cy.wrapping_sub(1), cy > 0),
                            (cx, cy + 1, cy + 1 < self.height),
                        ];
                        for (nx, ny, ok) in neighbors {
                            if !ok {
                                continue;
                            }
                            let nidx = self.idx(nx, ny);
                            if !visited[nidx] {
                                if let Some(Block::Garbage { .. }) = self.cells[nidx] {
                                    visited[nidx] = true;
                                    stack.push((nx, ny));
                                }
                            }
                        }
                    }

                    let mut in_component = vec![false; self.cells.len()];
                    for &(cx, cy) in &component {
                        in_component[self.idx(cx, cy)] = true;
                    }
                    let mut can_fall = true;
                    for &(cx, cy) in &component {
                        if cy == 0 {
                            can_fall = false;
                            break;
                        }
                        let below = self.idx(cx, cy - 1);
                        if self.cells[below].is_some() && !in_component[below] {
                            can_fall = false;
                            break;
                        }
                    }
                    if can_fall {
                        return true;
                    }
                }
            }
        }
        false
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
        match (self.get(ax, ay).and_then(Block::color), self.get(bx, by).and_then(Block::color)) {
            (Some(a), Some(b)) => a == b,
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
            self.cells[idx] = Some(Block::Normal { color });
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

        let horiz_left = left1.and_then(Block::color).map(|b| b == color).unwrap_or(false)
            && left2.and_then(Block::color).map(|b| b == color).unwrap_or(false);
        let horiz_right = right1.and_then(Block::color).map(|b| b == color).unwrap_or(false)
            && right2.and_then(Block::color).map(|b| b == color).unwrap_or(false);
        let horiz_split = left1.and_then(Block::color).map(|b| b == color).unwrap_or(false)
            && right1.and_then(Block::color).map(|b| b == color).unwrap_or(false);

        if horiz_left || horiz_right || horiz_split {
            return true;
        }

        if y + 2 < self.height {
            let up1 = self
                .get(x, y + 1)
                .and_then(Block::color)
                .map(|b| b == color)
                .unwrap_or(false);
            let up2 = self
                .get(x, y + 2)
                .and_then(Block::color)
                .map(|b| b == color)
                .unwrap_or(false);
            if up1 && up2 {
                return true;
            }
        }

        false
    }

    fn count_match_groups(&self, marks: &[bool]) -> u32 {
        let mut visited = vec![false; marks.len()];
        let mut groups = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if !marks[idx] || visited[idx] {
                    continue;
                }
                groups += 1;
                let mut stack = vec![(x, y)];
                visited[idx] = true;
                while let Some((cx, cy)) = stack.pop() {
                    let neighbors = [
                        (cx.wrapping_sub(1), cy, cx > 0),
                        (cx + 1, cy, cx + 1 < self.width),
                        (cx, cy.wrapping_sub(1), cy > 0),
                        (cx, cy + 1, cy + 1 < self.height),
                    ];
                    for (nx, ny, ok) in neighbors {
                        if !ok {
                            continue;
                        }
                        let nidx = self.idx(nx, ny);
                        if marks[nidx] && !visited[nidx] {
                            visited[nidx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }
        }
        groups
    }

    pub fn crack_adjacent_garbage(&mut self, marks: &[bool]) -> u32 {
        let mut cracked = 0;
        let mut visited = vec![false; self.cells.len()];
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                if visited[idx] {
                    continue;
                }
                if let Some(Block::Garbage { .. }) = self.cells[idx] {
                    let mut stack = vec![(x, y)];
                    let mut component: Vec<(usize, usize)> = Vec::new();
                    visited[idx] = true;
                    let mut adjacent = false;
                    while let Some((cx, cy)) = stack.pop() {
                        component.push((cx, cy));
                        if self.has_adjacent_mark(cx, cy, marks) {
                            adjacent = true;
                        }
                        let neighbors = [
                            (cx.wrapping_sub(1), cy, cx > 0),
                            (cx + 1, cy, cx + 1 < self.width),
                            (cx, cy.wrapping_sub(1), cy > 0),
                            (cx, cy + 1, cy + 1 < self.height),
                        ];
                        for (nx, ny, ok) in neighbors {
                            if !ok {
                                continue;
                            }
                            let nidx = self.idx(nx, ny);
                            if !visited[nidx] {
                                if let Some(Block::Garbage { .. }) = self.cells[nidx] {
                                    visited[nidx] = true;
                                    stack.push((nx, ny));
                                }
                            }
                        }
                    }

                    if adjacent {
                        for (cx, cy) in component {
                            if let Some(Block::Garbage { cracked: false }) = self.get(cx, cy) {
                                self.set(cx, cy, Some(Block::Garbage { cracked: true }));
                                cracked += 1;
                            }
                        }
                    }
                }
            }
        }
        cracked
    }

    fn has_adjacent_mark(&self, x: usize, y: usize, marks: &[bool]) -> bool {
        let neighbors = [
            (x.wrapping_sub(1), y, x > 0),
            (x + 1, y, x + 1 < self.width),
            (x, y.wrapping_sub(1), y > 0),
            (x, y + 1, y + 1 < self.height),
        ];
        for (nx, ny, ok) in neighbors {
            if !ok {
                continue;
            }
            if marks[self.idx(nx, ny)] {
                return true;
            }
        }
        false
    }

    pub fn convert_cracked_garbage(&mut self) -> u32 {
        let mut rng = thread_rng();
        let mut converted = 0;
        for y in 0..self.height {
            for x in 0..self.width {
                if let Some(Block::Garbage { cracked: true }) = self.get(x, y) {
                    let mut color = random_color(&mut rng);
                    for _ in 0..10 {
                        if !self.would_create_match(x, y, color) {
                            break;
                        }
                        color = random_color(&mut rng);
                    }
                    self.set(x, y, Some(Block::Normal { color }));
                    converted += 1;
                }
            }
        }
        converted
    }

    pub fn insert_garbage_rows_from_top(&mut self, rows: &[Vec<bool>]) -> bool {
        if rows.is_empty() {
            return true;
        }
        if rows.len() > self.height {
            return false;
        }
        for row in rows {
            if row.len() != self.width {
                return false;
            }
        }

        let start_y = self.height - rows.len();
        for (row_idx, row) in rows.iter().enumerate() {
            let y = start_y + row_idx;
            for x in 0..self.width {
                if row[x] && self.get(x, y).is_some() {
                    return false;
                }
            }
        }

        for (row_idx, row) in rows.iter().enumerate() {
            let y = start_y + row_idx;
            for x in 0..self.width {
                if row[x] {
                    self.set(x, y, Some(Block::Garbage { cracked: false }));
                }
            }
        }
        true
    }
}

pub struct ClearStats {
    pub cleared: u32,
    pub groups: u32,
    pub marks: Vec<bool>,
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
