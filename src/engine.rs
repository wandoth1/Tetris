//! Deterministic falling-block engine. Positive Y points DOWN.
//! Four hidden rows allow wall/floor kicks without out-of-bounds indexing.
use std::collections::VecDeque;

pub const WIDTH: usize = 10;
pub const HIDDEN: usize = 4;
pub const VISIBLE: usize = 20;
pub const HEIGHT: usize = HIDDEN + VISIBLE;
pub const LOCK_DELAY: f64 = 0.5;
pub const MAX_LOCK_RESETS: u8 = 15;
pub const CLEAR_DELAY: f64 = 0.18;
pub type Board = [[u8; WIDTH]; HEIGHT];
pub type Cells = [(i32, i32); 4];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind { I = 1, O, T, S, Z, J, L }
impl Kind {
    pub const ALL: [Self; 7] = [Self::I, Self::O, Self::T, Self::S, Self::Z, Self::J, Self::L];
    pub fn from_cell(value: u8) -> Option<Self> { Self::ALL.get(value.wrapping_sub(1) as usize).copied() }
    pub fn offsets(self, rotation: u8) -> Cells {
        let r = rotation % 4;
        if self == Self::I {
            return match r {
                0 => [(0, 1), (1, 1), (2, 1), (3, 1)],
                1 => [(2, 0), (2, 1), (2, 2), (2, 3)],
                2 => [(0, 2), (1, 2), (2, 2), (3, 2)],
                _ => [(1, 0), (1, 1), (1, 2), (1, 3)],
            };
        }
        if self == Self::O { return [(1, 0), (2, 0), (1, 1), (2, 1)]; }
        let mut cells = match self {
            Self::T => [(1, 0), (0, 1), (1, 1), (2, 1)],
            Self::S => [(1, 0), (2, 0), (0, 1), (1, 1)],
            Self::Z => [(0, 0), (1, 0), (1, 1), (2, 1)],
            Self::J => [(0, 0), (0, 1), (1, 1), (2, 1)],
            Self::L => [(2, 0), (0, 1), (1, 1), (2, 1)],
            _ => unreachable!(),
        };
        for _ in 0..r { for p in &mut cells { *p = (2 - p.1, p.0); } }
        cells
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece { pub kind: Kind, pub rotation: u8, pub x: i32, pub y: i32 }
impl Piece {
    pub fn spawn(kind: Kind) -> Self { Self { kind, rotation: 0, x: 3, y: HIDDEN as i32 - 1 } }
    pub fn cells(self) -> Cells { self.kind.offsets(self.rotation).map(|(x, y)| (x + self.x, y + self.y)) }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Marathon, Sprint, Zen }
impl Mode {
    pub const ALL: [Self; 3] = [Self::Marathon, Self::Sprint, Self::Zen];
    pub fn index(self) -> usize { match self { Self::Marathon => 0, Self::Sprint => 1, Self::Zen => 2 } }
    pub fn name(self) -> &'static str { match self { Self::Marathon => "MARATON", Self::Sprint => "40 LINEAS", Self::Zen => "ZEN" } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spin { None, Mini, Full }
#[derive(Clone, Debug)]
pub enum Event {
    Lock { cells: Cells, kind: Kind },
    Clear { rows: Vec<usize>, points: u64, spin: Spin, combo: i32, back_to_back: bool, perfect: bool },
    Spin { points: u64, mini: bool },
    LevelUp(u32),
    Finish { won: bool },
}
#[derive(Clone, Debug)]
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Self(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed }) }
    fn next(&mut self) -> u64 {
        let mut x = self.0; x ^= x >> 12; x ^= x << 25; x ^= x >> 27;
        self.0 = x; x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: u64) -> usize {
        let threshold = n.wrapping_neg() % n;
        loop { let x = self.next(); if x >= threshold { return (x % n) as usize; } }
    }
}
#[derive(Clone, Debug)]
struct Bag { queue: VecDeque<Kind>, rng: Rng }
impl Bag {
    fn new(seed: u64) -> Self {
        let mut bag = Self { queue: VecDeque::new(), rng: Rng::new(seed) }; bag.refill(); bag
    }
    fn refill(&mut self) {
        while self.queue.len() < 8 {
            let mut values = Kind::ALL;
            for i in (1..values.len()).rev() { let j = self.rng.below((i + 1) as u64); values.swap(i, j); }
            self.queue.extend(values);
        }
    }
    fn take(&mut self) -> Kind {
        self.refill(); let piece = self.queue.pop_front().expect("refill guarantees a nonempty bag");
        self.refill(); piece
    }
}
#[derive(Clone, Debug)]
struct PendingClear { rows: Vec<usize>, remaining: f64 }
#[derive(Clone, Debug)]
pub struct Game {
    pub board: Board, pub active: Piece, pub held: Option<Kind>, pub hold_used: bool,
    pub mode: Mode, pub score: u64, pub lines: u32, pub level: u32, pub elapsed: f64,
    pub pieces: u32, pub combo: i32, pub back_to_back: bool, pub over: bool, pub won: bool,
    bag: Bag, gravity: f64, lock_time: f64, lock_resets: u8, touched_ground: bool,
    last_rotation: Option<usize>, pending: Option<PendingClear>, events: Vec<Event>,
}
impl Game {
    pub fn new(mode: Mode, seed: u64) -> Self {
        let mut bag = Bag::new(seed); let active = Piece::spawn(bag.take());
        Self {
            board: [[0; WIDTH]; HEIGHT], active, held: None, hold_used: false,
            mode, score: 0, lines: 0, level: 1, elapsed: 0.0, pieces: 0,
            combo: -1, back_to_back: false, over: false, won: false,
            bag, gravity: 0.0, lock_time: 0.0, lock_resets: 0,
            touched_ground: false, last_rotation: None, pending: None, events: Vec::new(),
        }
    }
    pub fn next_pieces(&self) -> impl Iterator<Item = Kind> + '_ { self.bag.queue.iter().copied().take(5) }
    pub fn take_events(&mut self) -> Vec<Event> { std::mem::take(&mut self.events) }
    pub fn clear_rows(&self) -> &[usize] { self.pending.as_ref().map_or(&[], |p| p.rows.as_slice()) }
    pub fn is_active(&self) -> bool { !self.over && self.pending.is_none() }
    pub fn fits(&self, p: Piece) -> bool {
        p.cells().iter().all(|&(x, y)| x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 && self.board[y as usize][x as usize] == 0)
    }
    pub fn grounded(&self) -> bool { !self.fits(Piece { y: self.active.y + 1, ..self.active }) }
    pub fn ghost(&self) -> Piece {
        let mut p = self.active; if !self.fits(p) { return p; }
        while self.fits(Piece { y: p.y + 1, ..p }) { p.y += 1; } p
    }
    pub fn lock_fraction(&self) -> f32 { (self.lock_time / LOCK_DELAY).clamp(0.0, 1.0) as f32 }
    pub fn interval(&self) -> f64 {
        match self.mode { Mode::Zen => f64::INFINITY, Mode::Sprint => 0.8,
            Mode::Marathon => (0.8 * 0.78_f64.powi((self.level.min(30) - 1) as i32)).max(0.035) }
    }
    fn reset_after_action(&mut self, was_grounded: bool) {
        if (was_grounded || self.touched_ground) && self.lock_resets < MAX_LOCK_RESETS {
            self.lock_time = 0.0; self.lock_resets += 1;
        }
    }
    pub fn shift(&mut self, dx: i32) -> bool {
        if !self.is_active() || ![-1, 1].contains(&dx) { return false; }
        let candidate = Piece { x: self.active.x + dx, ..self.active };
        if !self.fits(candidate) { return false; }
        let grounded = self.grounded(); self.active = candidate; self.last_rotation = None;
        self.reset_after_action(grounded); true
    }
    pub fn rotate(&mut self, clockwise: bool) -> bool {
        if !self.is_active() || self.active.kind == Kind::O { return false; }
        let from = self.active.rotation; let to = (from + if clockwise { 1 } else { 3 }) % 4;
        for (index, &(dx, dy)) in kicks(self.active.kind, from, to).iter().enumerate() {
            let candidate = Piece { rotation: to, x: self.active.x + dx, y: self.active.y + dy, ..self.active };
            if self.fits(candidate) {
                let grounded = self.grounded(); self.active = candidate; self.last_rotation = Some(index);
                self.reset_after_action(grounded); return true;
            }
        } false
    }
    pub fn soft_drop(&mut self) -> bool {
        if !self.is_active() { return false; }
        if self.step_down() { self.score += 1; self.last_rotation = None; true } else { false }
    }
    fn step_down(&mut self) -> bool {
        let p = Piece { y: self.active.y + 1, ..self.active };
        if self.fits(p) {
            self.active = p;
            // Do not grant fresh lock delays by walking down a staircase.
            if !self.touched_ground { self.lock_time = 0.0; } true
        } else { false }
    }
    pub fn hard_drop(&mut self) -> bool {
        if !self.is_active() { return false; }
        let target = self.ghost(); let distance = (target.y - self.active.y).max(0) as u64;
        self.score += 2 * distance; self.active = target;
        if distance > 0 { self.last_rotation = None; }
        self.lock(); true
    }
    pub fn hold(&mut self) -> bool {
        if !self.is_active() || self.hold_used { return false; }
        let outgoing = self.active.kind; let incoming = self.held.unwrap_or_else(|| self.bag.take());
        self.held = Some(outgoing); self.hold_used = true; self.install(incoming); true
    }
    fn install(&mut self, kind: Kind) {
        self.active = Piece::spawn(kind); self.gravity = 0.0; self.lock_time = 0.0; self.lock_resets = 0;
        self.touched_ground = false; self.last_rotation = None;
        if !self.fits(self.active) { self.finish(false); }
    }
    fn advance(&mut self) {
        if self.mode == Mode::Sprint && self.lines >= 40 { self.finish(true); return; }
        self.hold_used = false; let kind = self.bag.take(); self.install(kind);
    }
    fn finish(&mut self, won: bool) {
        if self.over { return; } self.over = true; self.won = won; self.events.push(Event::Finish { won });
    }
    pub fn tick(&mut self, dt: f64) {
        if self.over || !dt.is_finite() || dt <= 0.0 { return; }
        let dt = dt.min(0.25); self.elapsed += dt;
        if let Some(p) = &mut self.pending {
            p.remaining -= dt;
            if p.remaining <= 0.0 {
                let rows = self.pending.take().expect("pending clear exists").rows;
                collapse(&mut self.board, &rows); self.advance();
            } return;
        }
        self.gravity += dt; let interval = self.interval();
        while self.gravity >= interval {
            self.gravity -= interval; if !self.step_down() { self.gravity = 0.0; break; }
        }
        if self.grounded() {
            self.touched_ground = true; self.lock_time += dt;
            if self.lock_time >= LOCK_DELAY { self.lock(); }
        }
    }
    fn spin(&self) -> Spin {
        if self.active.kind != Kind::T || self.last_rotation.is_none() { return Spin::None; }
        let x = self.active.x; let y = self.active.y;
        let blocked = |dx, dy| {
            let xx = x + dx; let yy = y + dy;
            xx < 0 || xx >= WIDTH as i32 || yy < 0 || yy >= HEIGHT as i32 || self.board[yy as usize][xx as usize] != 0
        };
        let corners = [blocked(0, 0), blocked(2, 0), blocked(0, 2), blocked(2, 2)];
        if corners.iter().filter(|&&v| v).count() < 3 { return Spin::None; }
        let front = match self.active.rotation { 0 => corners[0] && corners[1], 1 => corners[1] && corners[3], 2 => corners[2] && corners[3], _ => corners[0] && corners[2] };
        if front || self.last_rotation == Some(4) { Spin::Full } else { Spin::Mini }
    }
    fn lock(&mut self) {
        if !self.fits(self.active) { self.finish(false); return; }
        let spin = self.spin(); let cells = self.active.cells();
        for &(x, y) in &cells { self.board[y as usize][x as usize] = self.active.kind as u8; }
        self.pieces += 1; self.events.push(Event::Lock { cells, kind: self.active.kind });
        if cells.iter().all(|&(_, y)| y < HIDDEN as i32) { self.finish(false); return; }
        let rows: Vec<usize> = (0..HEIGHT).filter(|&y| self.board[y].iter().all(|&v| v != 0)).collect();
        let count = rows.len();
        if count == 0 {
            self.combo = -1;
            let points = match spin { Spin::None => 0, Spin::Mini => 100, Spin::Full => 400 } * self.level as u64;
            self.score += points;
            if points > 0 { self.events.push(Event::Spin { points, mini: spin == Spin::Mini }); }
            self.advance(); return;
        }
        self.combo += 1; let difficult = count == 4 || spin != Spin::None;
        let b2b_bonus = difficult && self.back_to_back;
        let base = match spin { Spin::None => [0, 100, 300, 500, 800][count.min(4)],
            Spin::Mini => [100, 200, 400, 600, 800][count.min(4)], Spin::Full => [400, 800, 1200, 1600, 2000][count.min(4)] };
        let mut points = base * self.level as u64;
        if b2b_bonus { points = points * 3 / 2; }
        points += 50 * self.combo.max(0) as u64 * self.level as u64;
        let perfect = (0..HEIGHT).all(|y| rows.contains(&y) || self.board[y].iter().all(|&v| v == 0));
        if perfect { points += [0, 800, 1200, 1800, 2000][count.min(4)] * self.level as u64; }
        self.score += points; self.back_to_back = difficult; self.lines += count as u32;
        self.events.push(Event::Clear { rows: rows.clone(), points, spin, combo: self.combo, back_to_back: b2b_bonus, perfect });
        let new_level = if self.mode == Mode::Marathon { 1 + self.lines / 10 } else { 1 };
        if new_level > self.level { self.level = new_level; self.events.push(Event::LevelUp(self.level)); }
        self.pending = Some(PendingClear { rows, remaining: CLEAR_DELAY });
    }
}
fn collapse(board: &mut Board, rows: &[usize]) {
    let mut result = [[0; WIDTH]; HEIGHT]; let mut dest = HEIGHT;
    for source in (0..HEIGHT).rev() { if !rows.contains(&source) { dest -= 1; result[dest] = board[source]; } }
    *board = result;
}
// SRS offsets converted from upward-positive mathematical Y into screen Y.
fn kicks(kind: Kind, from: u8, to: u8) -> [(i32, i32); 5] {
    if kind == Kind::I {
        match (from, to) {
            (0, 1) | (3, 2) => [(0, 0), (-2, 0), (1, 0), (-2, 1), (1, -2)],
            (1, 0) | (2, 3) => [(0, 0), (2, 0), (-1, 0), (2, -1), (-1, 2)],
            (1, 2) | (0, 3) => [(0, 0), (-1, 0), (2, 0), (-1, -2), (2, 1)],
            (2, 1) | (3, 0) => [(0, 0), (1, 0), (-2, 0), (1, 2), (-2, -1)],
            _ => [(0, 0); 5],
        }
    } else {
        match (from, to) {
            (0, 1) | (2, 1) => [(0, 0), (-1, 0), (-1, -1), (0, 2), (-1, 2)],
            (1, 0) | (1, 2) => [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
            (2, 3) | (0, 3) => [(0, 0), (1, 0), (1, -1), (0, 2), (1, 2)],
            (3, 2) | (3, 0) => [(0, 0), (-1, 0), (-1, 1), (0, -2), (-1, -2)],
            _ => [(0, 0); 5],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game { Game::new(Mode::Marathon, 42) }
    fn set_piece(g: &mut Game, kind: Kind, rotation: u8, x: i32, y: i32) {
        g.active = Piece { kind, rotation, x, y }; g.last_rotation = None; g.pending = None; g.over = false;
    }
    fn tetris_well(g: &mut Game) {
        g.board = [[0; WIDTH]; HEIGHT];
        for y in HEIGHT - 4..HEIGHT { g.board[y] = [Kind::J as u8; WIDTH]; g.board[y][4] = 0; }
        set_piece(g, Kind::I, 1, 2, HEIGHT as i32 - 4);
    }
    #[test] fn every_rotation_has_four_unique_cells() { for kind in Kind::ALL { for r in 0..4 { let mut cells = kind.offsets(r).to_vec(); cells.sort(); cells.dedup(); assert_eq!(cells.len(), 4); } } }
    #[test] fn pieces_spawn_without_collision() { let g = game(); for k in Kind::ALL { assert!(g.fits(Piece::spawn(k))); } }
    #[test] fn cells_stay_in_four_by_four_box() { for k in Kind::ALL { for r in 0..4 { for (x, y) in k.offsets(r) { assert!((0..4).contains(&x) && (0..4).contains(&y)); } } } }
    #[test] fn seven_bag_has_each_piece_once() { let mut b = Bag::new(123); for _ in 0..1000 { let mut batch = [0; 7]; for v in &mut batch { *v = b.take() as u8; } batch.sort(); assert_eq!(batch, [1, 2, 3, 4, 5, 6, 7]); } }
    #[test] fn zero_seed_works() { let mut b = Bag::new(0); let first = b.take(); assert!((0..20).any(|_| b.take() != first)); }
    #[test] fn bag_is_deterministic() { let (mut a, mut b) = (Bag::new(71), Bag::new(71)); for _ in 0..200 { assert_eq!(a.take(), b.take()); } }
    #[test] fn preview_always_has_five_pieces() { let mut b = Bag::new(2); for _ in 0..100 { b.take(); assert!(b.queue.len() >= 5); } }
    #[test] fn walls_and_floor_collide() { let g = game(); let p = Piece::spawn(Kind::T); assert!(!g.fits(Piece { x: -1, ..p })); assert!(!g.fits(Piece { x: 9, ..p })); assert!(!g.fits(Piece { y: HEIGHT as i32 - 1, ..p })); }
    #[test] fn negative_y_is_safely_rejected() { let g = game(); assert!(!g.fits(Piece { y: -2, ..Piece::spawn(Kind::T) })); }
    #[test] fn stack_collision() { let mut g = game(); let (x, y) = g.active.cells()[0]; g.board[y as usize][x as usize] = 1; assert!(!g.fits(g.active)); }
    #[test] fn shift_stops_at_wall() { let mut g = game(); for _ in 0..30 { g.shift(-1); } assert!(g.fits(g.active)); assert!(!g.shift(-1)); }
    #[test] fn rejects_large_shift() { let mut g = game(); assert!(!g.shift(10)); assert!(!g.shift(0)); }
    #[test] fn rotations_round_trip() { let mut g = game(); for k in [Kind::I, Kind::T, Kind::S, Kind::Z, Kind::J, Kind::L] { g.active = Piece::spawn(k); let original = g.active; for _ in 0..4 { assert!(g.rotate(true)); } assert_eq!(g.active, original); } }
    #[test] fn square_rotation_does_not_reset_lock() { let mut g = game(); g.active = Piece::spawn(Kind::O); assert!(!g.rotate(true)); assert_eq!(g.lock_resets, 0); }
    #[test] fn counterclockwise_rotation() { let mut g = game(); g.active = Piece::spawn(Kind::T); assert!(g.rotate(false)); assert_eq!(g.active.rotation, 3); }
    #[test] fn i_piece_kicks_at_left_wall() { let mut g = game(); set_piece(&mut g, Kind::I, 1, -2, 10); assert!(g.fits(g.active)); assert!(g.rotate(false)); assert_eq!(g.active.x, 0); }
    #[test] fn t_piece_floor_kick() { let mut g = game(); set_piece(&mut g, Kind::T, 0, 3, HEIGHT as i32 - 2); assert!(g.rotate(true)); assert!(g.fits(g.active)); assert!(g.active.y < HEIGHT as i32 - 2); }
    #[test] fn ghost_is_last_valid_position() { let g = game(); let p = g.ghost(); assert!(g.fits(p)); assert!(!g.fits(Piece { y: p.y + 1, ..p })); }
    #[test] fn hard_drop_locks_and_scores_distance() { let mut g = game(); let d = g.ghost().y - g.active.y; assert!(g.hard_drop()); assert_eq!(g.score, 2 * d as u64); assert_eq!(g.pieces, 1); }
    #[test] fn soft_drop_scores_one() { let mut g = game(); assert!(g.soft_drop()); assert_eq!(g.score, 1); }
    #[test] fn held_piece_only_once_per_lock() { let mut g = game(); let first = g.active.kind; assert!(g.hold()); assert_eq!(g.held, Some(first)); assert!(!g.hold()); g.hard_drop(); assert!(g.hold()); }
    #[test] fn hold_resets_orientation() { let mut g = game(); g.held = Some(Kind::T); g.hold(); assert_eq!(g.active, Piece::spawn(Kind::T)); }
    #[test] fn hold_swap_does_not_consume_queue() { let mut g = game(); g.held = Some(Kind::T); let before: Vec<_> = g.next_pieces().collect(); g.hold(); assert_eq!(before, g.next_pieces().collect::<Vec<_>>()); }
    #[test] fn gravity_advances_piece() { let mut g = game(); let y = g.active.y; for _ in 0..10 { g.tick(0.1); } assert!(g.active.y > y); }
    #[test] fn zen_has_no_automatic_gravity() { let mut g = Game::new(Mode::Zen, 1); let p = g.active; for _ in 0..200 { g.tick(0.1); } assert_eq!(g.active, p); }
    #[test] fn invalid_dt_does_not_corrupt_clock() { let mut g = game(); g.tick(f64::NAN); g.tick(f64::INFINITY); g.tick(-1.0); assert_eq!(g.elapsed, 0.0); }
    #[test] fn lock_delay_is_not_instant() { let mut g = game(); g.active = g.ghost(); g.tick(0.2); assert_eq!(g.pieces, 0); g.tick(0.2); g.tick(0.2); assert_eq!(g.pieces, 1); }
    #[test] fn lock_resets_are_bounded() { let mut g = Game::new(Mode::Zen, 1); g.active = Piece::spawn(Kind::O); g.active = g.ghost(); for i in 0..100 { if g.pieces > 0 { break; } g.shift(if i % 2 == 0 { -1 } else { 1 }); g.tick(0.05); } assert_eq!(g.pieces, 1); }
    #[test] fn failed_action_does_not_reset_lock() { let mut g = game(); g.active = g.ghost(); for _ in 0..20 { g.shift(-1); } g.lock_time = 0.4; assert!(!g.shift(-1)); assert_eq!(g.lock_time, 0.4); }
    #[test] fn four_lines_perfect_clear_scores_2800() { let mut g = game(); tetris_well(&mut g); g.hard_drop(); assert_eq!(g.lines, 4); assert_eq!(g.score, 2800); assert_eq!(g.clear_rows().len(), 4); g.tick(0.2); assert!(g.board.iter().flatten().all(|&v| v == 0)); }
    #[test] fn pending_clear_rejects_inputs() { let mut g = game(); tetris_well(&mut g); g.hard_drop(); assert!(!g.hard_drop()); assert!(!g.hold()); assert!(!g.shift(1)); assert!(!g.rotate(true)); }
    #[test] fn back_to_back_and_combo_bonus() { let mut g = game(); tetris_well(&mut g); g.hard_drop(); g.tick(0.2); tetris_well(&mut g); g.hard_drop(); assert_eq!(g.score, 2800 + 1200 + 50 + 2000); }
    #[test] fn empty_lock_preserves_b2b_but_breaks_combo() { let mut g = game(); g.back_to_back = true; g.combo = 3; g.hard_drop(); assert!(g.back_to_back); assert_eq!(g.combo, -1); }
    #[test] fn single_clear_breaks_b2b() { let mut g = game(); g.back_to_back = true; g.board[HEIGHT - 1] = [1; WIDTH]; for x in 3..7 { g.board[HEIGHT - 1][x] = 0; } set_piece(&mut g, Kind::I, 0, 3, HEIGHT as i32 - 2); g.hard_drop(); assert!(!g.back_to_back); assert_eq!(g.lines, 1); }
    #[test] fn collapse_keeps_row_order() { let mut b = [[0; WIDTH]; HEIGHT]; b[HEIGHT - 1] = [1; WIDTH]; b[HEIGHT - 2][3] = 2; b[HEIGHT - 3][6] = 3; collapse(&mut b, &[HEIGHT - 1]); assert_eq!(b[HEIGHT - 1][3], 2); assert_eq!(b[HEIGHT - 2][6], 3); }
    #[test] fn collapse_handles_hidden_rows() { let mut b = [[0; WIDTH]; HEIGHT]; b[0][0] = 3; b[HEIGHT - 1] = [1; WIDTH]; collapse(&mut b, &[HEIGHT - 1]); assert_eq!(b[1][0], 3); assert_eq!(b[0][0], 0); }
    #[test] fn level_increases_each_ten_lines() { let mut g = game(); g.lines = 8; tetris_well(&mut g); g.hard_drop(); assert_eq!(g.level, 2); }
    #[test] fn sprint_wins_at_forty() { let mut g = Game::new(Mode::Sprint, 2); g.lines = 36; tetris_well(&mut g); g.hard_drop(); g.tick(0.2); assert!(g.won && g.over); }
    #[test] fn blocked_spawn_ends_game() { let mut g = game(); for row in &mut g.board[..HIDDEN + 2] { *row = [1; WIDTH]; } g.install(Kind::T); assert!(g.over); assert!(!g.won); }
    #[test] fn over_game_cannot_accept_input_or_time() { let mut g = game(); g.finish(false); g.tick(0.2); assert_eq!(g.elapsed, 0.0); assert!(!g.hold()); assert!(!g.hard_drop()); }
    #[test] fn t_spin_needs_last_rotation() { let mut g = game(); set_piece(&mut g, Kind::T, 0, 3, 10); for (x, y) in [(3, 10), (5, 10), (3, 12)] { g.board[y][x] = 1; } assert_eq!(g.spin(), Spin::None); g.last_rotation = Some(0); assert_eq!(g.spin(), Spin::Full); }
    #[test] fn t_spin_mini_uses_front_corners() { let mut g = game(); set_piece(&mut g, Kind::T, 0, 3, 10); for (x, y) in [(3, 10), (3, 12), (5, 12)] { g.board[y][x] = 1; } g.last_rotation = Some(0); assert_eq!(g.spin(), Spin::Mini); g.last_rotation = Some(4); assert_eq!(g.spin(), Spin::Full); }
    #[test] fn draining_events_is_one_shot() { let mut g = game(); g.hard_drop(); assert!(!g.take_events().is_empty()); assert!(g.take_events().is_empty()); }
    #[test] fn random_input_stress_preserves_invariants() {
        for seed in 1..32 { let mut g = Game::new(Mode::Marathon, seed); let mut rng = Rng::new(seed * 991);
            for _ in 0..3000 {
                match rng.below(7) { 0 => { g.shift(-1); }, 1 => { g.shift(1); }, 2 => { g.rotate(true); }, 3 => { g.rotate(false); }, 4 => { g.soft_drop(); }, 5 => { g.hard_drop(); }, _ => { g.hold(); } }
                g.tick(1.0 / 60.0); g.take_events(); assert!(g.board.iter().flatten().all(|&v| v <= 7));
                if g.is_active() { assert!(g.fits(g.active)); } if g.over { break; }
            }
        }
    }
}
