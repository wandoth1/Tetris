use macroquad::prelude::*;
use tetris_core::{engine::{Cells, Game, Kind, Mode, HIDDEN, HEIGHT, WIDTH}, storage::Records, synth::TRACKS};
pub const ACCENT: Color = Color::new(0.25, 0.94, 0.91, 1.0);
pub const WHITE_INK: Color = Color::new(0.90, 0.95, 1.0, 1.0);
pub const MUTED: Color = Color::new(0.47, 0.56, 0.69, 1.0);
const PINK: Color = Color::new(0.96, 0.34, 0.69, 1.0);
const PANEL: Color = Color::new(0.046, 0.064, 0.112, 0.96);
const BX: f32 = 390.0;
const BY: f32 = 92.0;
const CELL: f32 = 30.0;
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen { Menu, Playing, Paused, Finished }
pub struct Canvas { pub scale: f32, x: f32, y: f32 }
impl Canvas {
    pub fn new() -> Self {
        let scale = (screen_width() / 1080.0).min(screen_height() / 800.0).max(0.001);
        Self { scale, x: (screen_width() - 1080.0 * scale) / 2.0, y: (screen_height() - 800.0 * scale) / 2.0 }
    }
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, color: Color) { draw_rectangle(self.x + x * self.scale, self.y + y * self.scale, w * self.scale, h * self.scale, color); }
    pub fn outline(&self, x: f32, y: f32, w: f32, h: f32, line: f32, color: Color) { draw_rectangle_lines(self.x + x * self.scale, self.y + y * self.scale, w * self.scale, h * self.scale, line * self.scale, color); }
    pub fn line(&self, x: f32, y: f32, xx: f32, yy: f32, width: f32, color: Color) { draw_line(self.x + x * self.scale, self.y + y * self.scale, self.x + xx * self.scale, self.y + yy * self.scale, width * self.scale, color); }
    pub fn circle(&self, x: f32, y: f32, radius: f32, color: Color) { draw_circle(self.x + x * self.scale, self.y + y * self.scale, radius * self.scale, color); }
    pub fn text(&self, text: &str, x: f32, baseline: f32, size: f32, color: Color) { draw_text(text, self.x + x * self.scale, self.y + baseline * self.scale, size * self.scale, color); }
    pub fn center(&self, text: &str, x: f32, baseline: f32, size: f32, color: Color) {
        let width = measure_text(text, None, (size * self.scale).round().clamp(1.0, 65535.0) as u16, 1.0).width;
        draw_text(text, self.x + x * self.scale - width / 2.0, self.y + baseline * self.scale, size * self.scale, color);
    }
    pub fn hover(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        let (mx, my) = mouse_position(); let mx = (mx - self.x) / self.scale; let my = (my - self.y) / self.scale;
        mx >= x && mx <= x + w && my >= y && my <= y + h
    }
    pub fn clicked(&self, x: f32, y: f32, w: f32, h: f32) -> bool { is_mouse_button_pressed(MouseButton::Left) && self.hover(x, y, w, h) }
    fn panel(&self, x: f32, y: f32, w: f32, h: f32, label: &str) {
        self.rect(x, y, w, h, PANEL); self.outline(x, y, w, h, 1.0, alpha(ACCENT, 0.18));
        self.rect(x, y, 3.0, 24.0, ACCENT); self.text(label, x + 18.0, y + 28.0, 15.0, MUTED);
    }
    fn button(&self, x: f32, y: f32, w: f32, h: f32, label: &str, primary: bool) {
        let hovered = self.hover(x, y, w, h);
        self.rect(x, y, w, h, if primary { alpha(ACCENT, if hovered { 0.26 } else { 0.14 }) } else { PANEL });
        self.outline(x, y, w, h, 1.0, alpha(ACCENT, if hovered || primary { 0.8 } else { 0.3 }));
        self.center(label, x + w / 2.0, y + h / 2.0 + 7.0, 22.0, if primary { ACCENT } else { WHITE_INK });
    }
}
fn alpha(c: Color, a: f32) -> Color { Color { a, ..c } }
fn tint(c: Color, v: f32) -> Color { Color::new(c.r * v, c.g * v, c.b * v, c.a) }
fn color(kind: Kind) -> Color {
    match kind { Kind::I => ACCENT, Kind::O => Color::from_rgba(255, 205, 88, 255), Kind::T => Color::from_rgba(184, 112, 255, 255),
        Kind::S => Color::from_rgba(93, 233, 159, 255), Kind::Z => Color::from_rgba(255, 99, 131, 255),
        Kind::J => Color::from_rgba(104, 150, 255, 255), Kind::L => Color::from_rgba(255, 162, 96, 255) }
}
fn tile(c: &Canvas, x: f32, y: f32, size: f32, kind: Kind, ghost: bool, dim: bool) {
    let co = color(kind);
    if ghost {
        c.rect(x + 2.0, y + 2.0, size - 4.0, size - 4.0, alpha(co, 0.055));
        c.outline(x + 3.0, y + 3.0, size - 6.0, size - 6.0, 1.3, alpha(co, 0.46));
        c.rect(x + size / 2.0 - 1.0, y + size / 2.0 - 1.0, 2.0, 2.0, alpha(co, 0.6));
    } else {
        let value = if dim { 0.52 } else { 0.88 };
        c.rect(x + 1.0, y + 1.0, size - 2.0, size - 2.0, tint(co, 0.24));
        c.rect(x + 3.0, y + 3.0, size - 6.0, size - 6.0, tint(co, value));
        c.rect(x + 3.0, y + 3.0, size - 6.0, 2.0, alpha(WHITE_INK, if dim { 0.12 } else { 0.4 }));
        c.outline(x + 1.0, y + 1.0, size - 2.0, size - 2.0, 1.0, alpha(co, 0.8));
        c.rect(x + size - 7.0, y + size - 7.0, 2.0, 2.0, alpha(WHITE_INK, 0.35));
    }
}
fn preview(c: &Canvas, kind: Kind, center_x: f32, center_y: f32, size: f32, dim: bool) {
    let cells = kind.offsets(0);
    let min_x = cells.iter().map(|p| p.0).min().unwrap_or(0); let max_x = cells.iter().map(|p| p.0).max().unwrap_or(0);
    let min_y = cells.iter().map(|p| p.1).min().unwrap_or(0); let max_y = cells.iter().map(|p| p.1).max().unwrap_or(0);
    for (x, y) in cells { tile(c, center_x + (x - min_x) as f32 * size - (max_x - min_x + 1) as f32 * size / 2.0,
        center_y + (y - min_y) as f32 * size - (max_y - min_y + 1) as f32 * size / 2.0, size, kind, false, dim); }
}
pub fn background(c: &Canvas, time: f32, records: &Records) {
    clear_background(Color::from_rgba(6, 9, 19, 255));
    for i in 0..40 { let y = i as f32 * 20.0; c.rect(0.0, y, 1080.0, 20.0, Color::new(0.024 + i as f32 * 0.0004, 0.031, 0.064 + (i as f32 / 40.0) * 0.026, 1.0)); }
    let t = if records.reduced_motion { 0.0 } else { time };
    for i in 0..64 {
        let x = ((i * 173 + 39) % 1080) as f32; let y = ((i * 97 + 11) % 710) as f32;
        let a = 0.12 + 0.12 * (t * 0.65 + i as f32).sin().abs(); c.circle(x, y, if i % 4 == 0 { 1.4 } else { 0.8 }, alpha(WHITE_INK, a));
    }
    for i in -12..=12 { c.line(540.0 + i as f32 * 15.0, 630.0, 540.0 + i as f32 * 100.0, 800.0, 1.0, alpha(ACCENT, 0.07)); }
    for i in 0..9 { let y = 630.0 + (i as f32 / 8.0).powi(2) * 170.0; c.line(0.0, y, 1080.0, y, 1.0, alpha(ACCENT, 0.055)); }
    c.text("TETRIS", 42.0, 49.0, 31.0, WHITE_INK); c.text("/ NEON PULSE", 149.0, 49.0, 21.0, ACCENT);
    c.line(42.0, 66.0, 1038.0, 66.0, 1.0, alpha(ACCENT, 0.24)); c.text("WANDOTH  /  RUST EDITION", 803.0, 48.0, 14.0, MUTED);
}
pub fn menu(c: &Canvas, selected: usize, records: &Records, time: f32) {
    let pulse = if records.reduced_motion { 0.5 } else { 0.5 + 0.5 * (time * 1.5).sin() };
    for (i, k) in [Kind::J, Kind::S, Kind::T, Kind::I, Kind::L, Kind::O, Kind::Z].iter().enumerate() {
        preview(c, *k, 160.0 + i as f32 * 126.0, 163.0 + if records.reduced_motion { 0.0 } else { (time + i as f32).sin() * 5.0 }, 15.0, false);
    }
    c.center("NEON PULSE", 540.0, 258.0, 79.0, alpha(ACCENT, 0.08 + pulse * 0.09));
    c.center("NEON PULSE", 540.0, 254.0, 76.0, WHITE_INK);
    c.center("ENCUENTRA TU RITMO. ENCAJA EL CAOS.", 540.0, 301.0, 21.0, ACCENT);
    c.center("Tres formas de jugar. Una banda sonora escrita en Rust.", 540.0, 333.0, 17.0, MUTED);
    let titles = ["MARATON", "40 LINEAS", "ZEN"];
    let descriptions = [["El clasico, cada vez mas rapido.", "Haz combos. Supera tu record."],
        ["40 lineas contra el cronometro.", "Precision, velocidad y cabeza fria."], ["Sin caida automatica.", "Respira. Coloca. Experimenta."]];
    for i in 0..3 {
        let x = 112.0 + i as f32 * 294.0; let active = i == selected;
        c.rect(x, 373.0, 268.0, 135.0, if active { Color::from_rgba(17, 38, 49, 255) } else { PANEL });
        c.outline(x, 373.0, 268.0, 135.0, if active { 2.0 } else { 1.0 }, alpha(ACCENT, if active { 0.9 } else { 0.15 }));
        c.text(&format!("0{}", i + 1), x + 19.0, 400.0, 14.0, if active { ACCENT } else { MUTED });
        c.text(titles[i], x + 19.0, 434.0, 27.0, WHITE_INK);
        c.text(descriptions[i][0], x + 19.0, 463.0, 16.0, MUTED); c.text(descriptions[i][1], x + 19.0, 487.0, 16.0, MUTED);
        let best = if i == 1 { records.sprint_seconds.map_or("MEJOR TIEMPO  --".into(), |s| format!("MEJOR  {}", time_label(s))) }
            else { format!("RECORD  {}", records.best[i]) };
        c.center(&best, x + 134.0, 531.0, 14.0, MUTED);
    }
    c.button(378.0, 557.0, 324.0, 58.0, "JUGAR  /  ENTER", true);
    c.center("1 / 2 / 3 o flechas para elegir modo", 540.0, 645.0, 17.0, MUTED);
    c.center("PIEZA FANTASMA  /  RESERVA  /  COMBOS  /  T-SPINS", 540.0, 688.0, 15.0, alpha(ACCENT, 0.75));
}
#[derive(Clone)]
pub(crate) struct Particle { x: f32, y: f32, vx: f32, vy: f32, life: f32, max_life: f32, color: Color, size: f32 }
#[derive(Default)]
pub struct Fx { pub particles: Vec<Particle>, pub shake: f32, message: Option<(String, String, f32)>, pub level: Option<(u32, f32)>, serial: u32 }
impl Fx {
    fn burst(&mut self, x: f32, y: f32, co: Color, amount: usize) {
        for _ in 0..amount {
            self.serial = self.serial.wrapping_add(1); let n = self.serial.wrapping_mul(2_654_435_761);
            let angle = (n % 628) as f32 * 0.01; let speed = 35.0 + ((n >> 12) % 120) as f32;
            let life = 0.35 + ((n >> 20) % 50) as f32 / 100.0;
            self.particles.push(Particle { x, y, vx: angle.cos() * speed, vy: angle.sin() * speed - 35.0, life, max_life: life, color: co, size: 1.5 + (n % 3) as f32 });
        }
        if self.particles.len() > 900 { let count = self.particles.len() - 900; self.particles.drain(..count); }
    }
    pub fn lock(&mut self, cells: Cells, kind: Kind, reduced: bool) {
        if reduced { return; }
        for (x, y) in cells { if y >= HIDDEN as i32 { self.burst(BX + x as f32 * CELL + CELL / 2.0, BY + (y - HIDDEN as i32) as f32 * CELL + CELL / 2.0, color(kind), 6); } } self.shake = 2.0;
    }
    pub fn clear(&mut self, rows: &[usize], reduced: bool) {
        if reduced { return; }
        for &row in rows { if row >= HIDDEN { for x in 0..WIDTH { self.burst(BX + x as f32 * CELL + CELL / 2.0, BY + (row - HIDDEN) as f32 * CELL + CELL / 2.0, if rows.len() == 4 { PINK } else { ACCENT }, 7); } } }
        self.shake = if rows.len() >= 4 { 6.0 } else { 3.0 };
    }
    pub fn announce(&mut self, title: String, detail: String) { self.message = Some((title, detail, 2.6)); }
    pub fn tick(&mut self, dt: f32) {
        for p in &mut self.particles { p.life -= dt; p.x += p.vx * dt; p.y += p.vy * dt; p.vy += 130.0 * dt; }
        self.particles.retain(|p| p.life > 0.0); self.shake = (self.shake - dt * 20.0).max(0.0);
        if let Some((_, _, left)) = &mut self.message { *left -= dt; if *left <= 0.0 { self.message = None; } }
        if let Some((_, left)) = &mut self.level { *left -= dt; if *left <= 0.0 { self.level = None; } }
    }
}
fn time_label(seconds: f64) -> String {
    let hundredths = (seconds.max(0.0) * 100.0) as u64;
    format!("{:02}:{:02}.{:02}", hundredths / 6000, (hundredths / 100) % 60, hundredths % 100)
}
pub fn game(c: &Canvas, g: &Game, records: &Records, fx: &Fx, time: f32) {
    let shift = if records.reduced_motion { 0.0 } else { fx.shake * (time * 65.0).sin() }; let bx = BX + shift;
    for i in (1..=4).rev() { let p = i as f32 * 3.0; c.outline(bx - p, BY - p, 300.0 + 2.0 * p, 600.0 + 2.0 * p, 2.0, alpha(ACCENT, 0.025)); }
    c.rect(bx, BY, 300.0, 600.0, Color::from_rgba(7, 12, 23, 255));
    for row in 0..20 { for col in 0..10 { c.rect(bx + col as f32 * CELL + 1.0, BY + row as f32 * CELL + 1.0, CELL - 2.0, CELL - 2.0, Color::from_rgba(12, 19, 31, 255)); } }
    c.outline(bx - 1.0, BY - 1.0, 302.0, 602.0, 1.5, alpha(ACCENT, 0.6));
    for row in HIDDEN..HEIGHT { for col in 0..WIDTH { if let Some(k) = Kind::from_cell(g.board[row][col]) { tile(c, bx + col as f32 * CELL, BY + (row - HIDDEN) as f32 * CELL, CELL, k, false, g.over); } } }
    if g.is_active() {
        for (x, y) in g.ghost().cells() { if y >= HIDDEN as i32 { tile(c, bx + x as f32 * CELL, BY + (y - HIDDEN as i32) as f32 * CELL, CELL, g.active.kind, true, false); } }
        for (x, y) in g.active.cells() { if y >= HIDDEN as i32 { tile(c, bx + x as f32 * CELL, BY + (y - HIDDEN as i32) as f32 * CELL, CELL, g.active.kind, false, false); } }
        if g.grounded() { c.rect(bx, BY + 606.0, 300.0 * (1.0 - g.lock_fraction()), 3.0, alpha(color(g.active.kind), 0.7)); }
    }
    for &row in g.clear_rows() { if row >= HIDDEN { c.rect(bx, BY + (row - HIDDEN) as f32 * CELL, 300.0, CELL, alpha(WHITE_INK, if records.reduced_motion { 0.18 } else { 0.38 })); } }
    c.center(g.mode.name(), 540.0, 81.0, 14.0, MUTED);
    c.panel(60.0, 112.0, 276.0, 130.0, "RESERVA  /  C");
    if let Some(kind) = g.held { preview(c, kind, 198.0, 183.0, 25.0, g.hold_used); } else { c.center("GUARDA UNA PIEZA", 198.0, 188.0, 16.0, MUTED); }
    c.panel(60.0, 264.0, 276.0, 128.0, "PUNTUACION"); c.text(&format!("{:07}", g.score), 79.0, 333.0, 40.0, WHITE_INK);
    c.text(&format!("RECORD  {:07}", records.best[g.mode.index()].max(g.score)), 80.0, 370.0, 16.0, ACCENT);
    c.panel(60.0, 414.0, 276.0, 134.0, "ESTADO"); c.text(&format!("NIVEL  {:02}", g.level), 80.0, 474.0, 22.0, WHITE_INK);
    c.text(&format!("LINEAS  {:03}", g.lines), 80.0, 510.0, 22.0, WHITE_INK);
    let fraction = if g.mode == Mode::Sprint { (g.lines as f32 / 40.0).min(1.0) } else { (g.lines % 10) as f32 / 10.0 };
    c.rect(80.0, 530.0, 236.0, 3.0, alpha(ACCENT, 0.1)); c.rect(80.0, 530.0, 236.0 * fraction, 3.0, ACCENT);
    c.panel(60.0, 570.0, 276.0, 122.0, "CONTROLES"); c.text("Flechas / WASD   mover", 79.0, 626.0, 17.0, WHITE_INK);
    c.text("Z / X  girar   Espacio  caer", 79.0, 651.0, 16.0, MUTED); c.text("C  reserva      P / Esc  pausa", 79.0, 676.0, 16.0, MUTED);
    c.panel(744.0, 112.0, 276.0, 426.0, "SIGUIENTES  /  7-BAG");
    for (i, kind) in g.next_pieces().enumerate() { let y = 182.0 + i as f32 * 73.0;
        c.text(&format!("0{}", i + 1), 767.0, y + 6.0, 15.0, MUTED); preview(c, kind, 892.0, y, if i == 0 { 27.0 } else { 23.0 }, false);
        if i < 4 { c.line(767.0, y + 37.0, 997.0, y + 37.0, 1.0, alpha(ACCENT, 0.08)); }
    }
    c.panel(744.0, 560.0, 276.0, 132.0, "TIEMPO / FLUJO");
    if let Some((title, detail, _)) = &fx.message { c.text(title, 762.0, 625.0, 26.0, ACCENT); c.text(detail, 762.0, 657.0, 17.0, WHITE_INK); }
    else {
        c.text(&time_label(g.elapsed), 763.0, 626.0, 33.0, WHITE_INK);
        let label = if g.mode == Mode::Sprint { format!("OBJETIVO  {} / 40", g.lines.min(40)) }
            else if g.mode == Mode::Zen { "A TU RITMO. SIN PRISA.".into() } else { format!("{} PIEZAS  /  {} COMBO", g.pieces, (g.combo + 1).max(0)) };
        c.text(&label, 764.0, 661.0, 16.0, MUTED);
    }
    if let Some((level, _)) = fx.level { c.center(&format!("NIVEL {level}"), 540.0, 724.0, 21.0, ACCENT); }
    for p in &fx.particles { c.rect(p.x, p.y, p.size, p.size, alpha(p.color, (p.life / p.max_life).clamp(0.0, 1.0))); }
}
pub fn pause(c: &Canvas, restart_armed: bool) {
    c.rect(0.0, 70.0, 1080.0, 662.0, Color::new(0.018, 0.025, 0.05, 0.87)); c.panel(340.0, 216.0, 400.0, 358.0, "DESCONECTA UN MOMENTO");
    c.center("PAUSA", 540.0, 314.0, 59.0, WHITE_INK);
    c.center(if restart_armed { "Pulsa R otra vez para reiniciar." } else { "La siguiente pieza puede esperar." }, 540.0, 356.0, 20.0, MUTED);
    c.center("R  reiniciar    M  musica    F2  menos efectos", 540.0, 394.0, 16.0, MUTED);
    c.button(388.0, 425.0, 304.0, 48.0, "CONTINUAR / ENTER", true); c.button(388.0, 490.0, 304.0, 42.0, "MENU / Q", false);
}
pub fn finished(c: &Canvas, g: &Game) {
    c.rect(0.0, 70.0, 1080.0, 662.0, Color::new(0.018, 0.025, 0.05, 0.88));
    c.panel(340.0, 210.0, 400.0, 396.0, if g.won { "OBJETIVO COMPLETADO" } else { "CADA PARTIDA ES UN NUEVO RITMO" });
    c.center(if g.won { "40 LINEAS" } else { "GAME OVER" }, 540.0, 310.0, 48.0, if g.won { ACCENT } else { WHITE_INK });
    c.center(&format!("{} PUNTOS", g.score), 540.0, 365.0, 32.0, ACCENT);
    c.center(&format!("{} lineas  /  {}", g.lines, time_label(g.elapsed)), 540.0, 403.0, 20.0, MUTED);
    c.center("Una mas. Esta vez encaja.", 540.0, 440.0, 18.0, MUTED);
    c.button(388.0, 472.0, 304.0, 48.0, "OTRA PARTIDA / ENTER", true); c.button(388.0, 535.0, 304.0, 40.0, "MENU / ESC", false);
}
pub fn footer(c: &Canvas, records: &Records, available: bool, time: f32) {
    c.line(42.0, 741.0, 1038.0, 741.0, 1.0, alpha(ACCENT, 0.18)); let track = TRACKS[records.track % TRACKS.len()];
    for i in 0..16 {
        let h = if !available || !records.music { 2.0 } else if records.reduced_motion { 5.0 + (i % 4) as f32 * 2.0 }
            else { 3.0 + 17.0 * (time * track.bpm / 60.0 * std::f32::consts::PI + i as f32 * 0.8).sin().abs() };
        c.rect(45.0 + i as f32 * 4.5, 775.0 - h, 2.0, h, alpha(ACCENT, 0.8));
    }
    c.text(if !available { "AUDIO NO DISPONIBLE" } else if records.music { track.name } else { "MUSICA EN SILENCIO" }, 133.0, 769.0, 18.0, WHITE_INK);
    c.text(&format!("{} BPM", track.bpm as u32), 318.0, 769.0, 13.0, MUTED);
    c.text(&format!("M musica   N pista   V efectos   +/- {}%", (records.volume * 100.0).round() as u32), 431.0, 768.0, 15.0, MUTED);
    c.text("F2 menos efectos   F11 pantalla", 814.0, 768.0, 14.0, MUTED);
}
/// Fixed fixture for a real renderer screenshot, not a mockup.
pub fn demo_game() -> Game {
    let mut g = Game::new(Mode::Marathon, 314159);
    let rows = [
        [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0],
        [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0],
        [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0],
        [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,0,0], [0,0,0,0,0,0,0,0,7,0],
        [6,0,0,0,0,0,0,7,7,0], [6,6,6,0,0,0,4,4,7,0], [2,2,3,0,0,4,4,5,5,0], [2,2,3,3,3,1,1,1,1,0],
    ];
    for (i, row) in rows.into_iter().enumerate() { g.board[HIDDEN + i] = row; }
    g.active = tetris_core::engine::Piece { kind: Kind::T, rotation: 0, x: 3, y: 8 };
    g.held = Some(Kind::I); g.score = 12480; g.lines = 36; g.level = 4; g.elapsed = 183.42; g.pieces = 97; g
}
