#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#![forbid(unsafe_code)]
mod audio;
mod visual;
use audio::Audio;
use macroquad::prelude::*;
use std::{path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tetris_core::{engine::{Event, Game, Mode, Spin}, storage::{save_path, Records}, synth::TRACKS};
use visual::{Canvas, Fx, Screen, ACCENT, MUTED, WHITE_INK};

fn window_conf() -> Conf {
    Conf { window_title: "Tetris | Neon Pulse".to_owned(), window_width: 1080, window_height: 800,
        high_dpi: true, window_resizable: true, sample_count: 1, ..Default::default() }
}
fn seed() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map_or(42, |d| d.as_nanos() as u64) }
fn persist(records: &Records, path: Option<&PathBuf>) -> bool { path.is_some_and(|p| records.save(p).is_ok()) }
/// Frame-independent delayed auto shift: 150 ms delay, 40 ms repeat.
#[derive(Default)]
struct Repeater { direction: i32, remaining: f64 }
impl Repeater {
    fn reset(&mut self) { *self = Self::default(); }
    fn update(&mut self, game: &mut Game, dt: f64) -> bool {
        let left = is_key_down(KeyCode::Left) || is_key_down(KeyCode::A);
        let right = is_key_down(KeyCode::Right) || is_key_down(KeyCode::D);
        let new_left = is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A);
        let new_right = is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D);
        let direction = match (left, right) {
            (true, false) => -1, (false, true) => 1,
            (true, true) if new_left => -1, (true, true) if new_right => 1,
            (true, true) => self.direction, _ => 0,
        };
        if direction == 0 { self.reset(); return false; }
        if direction != self.direction { self.direction = direction; self.remaining = 0.15; return game.shift(direction); }
        self.remaining -= dt; let mut moved = false;
        while self.remaining <= 0.0 { moved |= game.shift(direction); self.remaining += 0.04; } moved
    }
}
#[macroquad::main(window_conf)]
async fn main() {
    prevent_quit();
    let args: Vec<String> = std::env::args().collect();
    let smoke = args.iter().any(|a| a == "--smoke-test");
    let screenshot = args.windows(2).find(|a| a[0] == "--screenshot").map(|a| a[1].clone());
    let path = if smoke || screenshot.is_some() { None } else { save_path() };
    let mut records = path.as_ref().map_or_else(Records::default, |p| Records::load(p));
    let mut storage_ok = path.is_some();
    clear_background(Color::from_rgba(8, 11, 23, 255));
    draw_text("NEON PULSE / preparando sintetizadores...", 60.0, 100.0, 25.0, WHITE_INK);
    next_frame().await;
    let mut audio = Audio::load().await;
    let mut game = Game::new(Mode::Marathon, seed());
    let mut screen = Screen::Menu; let mut selected = 0; let mut fx = Fx::default();
    let mut repeat = Repeater::default(); let mut soft_timer = 0.0; let mut fullscreen = false;
    let mut elapsed_visual = 0.0_f32; let mut frame = 0_u32; let mut records_timer = 0.0;
    let mut restart_armed = false;
    if smoke || screenshot.is_some() { game = visual::demo_game(); screen = Screen::Playing; }
    loop {
        let raw_dt = get_frame_time() as f64; let dt = raw_dt.clamp(0.0, 0.25);
        elapsed_visual += dt as f32; frame += 1;
        let c = Canvas::new(); let mut dirty = false; let mut skip_input = false; let mut quit = is_quit_requested();
        if is_key_pressed(KeyCode::F11) { fullscreen = !fullscreen; set_fullscreen(fullscreen); }
        if is_key_pressed(KeyCode::M) { records.music = !records.music; dirty = true; }
        if is_key_pressed(KeyCode::N) { records.track = (records.track + 1) % TRACKS.len(); dirty = true; }
        if is_key_pressed(KeyCode::V) { records.effects = !records.effects; dirty = true; }
        if is_key_pressed(KeyCode::F2) { records.reduced_motion = !records.reduced_motion; dirty = true; }
        if is_key_pressed(KeyCode::Equal) || is_key_pressed(KeyCode::KpAdd) { records.volume = (records.volume + 0.05).min(1.0); dirty = true; }
        if is_key_pressed(KeyCode::Minus) || is_key_pressed(KeyCode::KpSubtract) { records.volume = (records.volume - 0.05).max(0.0); dirty = true; }
        if screen == Screen::Playing && raw_dt > 0.5 && !smoke { screen = Screen::Paused; skip_input = true; }
        if !smoke && screenshot.is_none() {
            match screen {
                Screen::Menu => {
                    if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) { selected = (selected + 2) % 3; }
                    if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) { selected = (selected + 1) % 3; }
                    for (i, key) in [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3].iter().enumerate() {
                        if is_key_pressed(*key) || c.clicked(112.0 + i as f32 * 294.0, 373.0, 268.0, 135.0) { selected = i; }
                    }
                    if is_key_pressed(KeyCode::Enter) || c.clicked(378.0, 557.0, 324.0, 58.0) {
                        game = Game::new(Mode::ALL[selected], seed()); screen = Screen::Playing;
                        fx = Fx::default(); repeat.reset(); soft_timer = 0.0; skip_input = true; audio.effect(5, &records);
                    }
                    if is_key_pressed(KeyCode::Escape) { quit = true; }
                }
                Screen::Playing => {
                    if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) {
                        screen = Screen::Paused; restart_armed = false; skip_input = true; records.observe(&game); dirty = true;
                    }
                }
                Screen::Paused => {
                    if !skip_input && (is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Enter) || c.clicked(388.0, 425.0, 304.0, 48.0)) {
                        screen = Screen::Playing; repeat.reset(); soft_timer = 0.0; skip_input = true; restart_armed = false;
                    }
                    if is_key_pressed(KeyCode::R) {
                        if restart_armed {
                            records.observe(&game); dirty = true; game = Game::new(game.mode, seed()); screen = Screen::Playing;
                            fx = Fx::default(); repeat.reset(); skip_input = true; restart_armed = false;
                        } else { restart_armed = true; }
                    }
                    if is_key_pressed(KeyCode::Q) || c.clicked(388.0, 490.0, 304.0, 42.0) {
                        records.observe(&game); dirty = true; screen = Screen::Menu; restart_armed = false;
                    }
                }
                Screen::Finished => {
                    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::R) || c.clicked(388.0, 472.0, 304.0, 48.0) {
                        game = Game::new(game.mode, seed()); screen = Screen::Playing; fx = Fx::default(); repeat.reset(); skip_input = true;
                    }
                    if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) || c.clicked(388.0, 535.0, 304.0, 40.0) { screen = Screen::Menu; }
                }
            }
            if screen == Screen::Playing && !skip_input {
                // Hold/hard-drop are exclusive actions: do not manipulate the next piece too.
                if is_key_pressed(KeyCode::C) || is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift) {
                    if game.hold() { audio.effect(5, &records); } repeat.reset(); soft_timer = 0.0;
                } else if is_key_pressed(KeyCode::Space) { game.hard_drop(); repeat.reset(); soft_timer = 0.0; }
                else {
                    if repeat.update(&mut game, dt) { audio.effect(0, &records); }
                    if (is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::X) || is_key_pressed(KeyCode::W)) && game.rotate(true) { audio.effect(1, &records); }
                    if is_key_pressed(KeyCode::Z) && game.rotate(false) { audio.effect(1, &records); }
                    if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
                        soft_timer -= dt; while soft_timer <= 0.0 { game.soft_drop(); soft_timer += 0.03; }
                    } else { soft_timer = 0.0; }
                }
                let mut remaining = dt;
                while remaining > 0.0 { let step = remaining.min(1.0 / 120.0); game.tick(step); remaining -= step; }
            }
        }
        for event in game.take_events() {
            match event {
                Event::Lock { cells, kind } => { fx.lock(cells, kind, records.reduced_motion); audio.effect(2, &records); }
                Event::Clear { rows, points, spin, combo, back_to_back, perfect } => {
                    fx.clear(&rows, records.reduced_motion);
                    let label = if perfect { "PERFECT CLEAR".to_owned() }
                        else if spin != Spin::None { format!("T-SPIN{}", if spin == Spin::Mini { " MINI" } else { "" }) }
                        else { ["", "SINGLE", "DOUBLE", "TRIPLE", "FOUR LINES"][rows.len().min(4)].to_owned() };
                    let detail = format!("+{points}{}{}", if back_to_back { "  B2B" } else { "" }, if combo > 0 { format!("  COMBO x{}", combo + 1) } else { String::new() });
                    fx.announce(label, detail); audio.effect(if rows.len() == 4 || perfect { 4 } else { 3 }, &records);
                }
                Event::Spin { points, mini } => { fx.announce(if mini { "T-SPIN MINI" } else { "T-SPIN" }.into(), format!("+{points}")); }
                Event::LevelUp(level) => { fx.level = Some((level, 2.0)); }
                Event::Finish { won } => { screen = Screen::Finished; records.observe(&game); dirty = true; audio.effect(if won { 7 } else { 6 }, &records); }
            }
        }
        fx.tick(dt as f32);
        if records.reduced_motion { fx.particles.clear(); fx.shake = 0.0; }
        records_timer += dt;
        if records_timer >= 5.0 && screen == Screen::Playing { let old_best = records.best; records.observe(&game); dirty |= old_best != records.best; records_timer = 0.0; }
        if dirty && path.is_some() { storage_ok = persist(&records, path.as_ref()); }
        audio.sync(&records, screen == Screen::Paused || screen == Screen::Finished);
        visual::background(&c, elapsed_visual, &records);
        if screen == Screen::Menu { visual::menu(&c, selected, &records, elapsed_visual); }
        else { visual::game(&c, &game, &records, &fx, elapsed_visual); if screen == Screen::Paused { visual::pause(&c, restart_armed); } if screen == Screen::Finished { visual::finished(&c, &game); } }
        visual::footer(&c, &records, audio.available, elapsed_visual);
        if !storage_ok && !smoke && screenshot.is_none() { c.text("Aviso: los records no se pueden guardar en este equipo.", 44.0, 722.0, 15.0, MUTED); }
        if smoke || screenshot.is_some() {
            c.text("DEMO / RENDER CHECK", 736.0, 58.0, 14.0, ACCENT);
            if frame == 8 { if let Some(file) = &screenshot { get_screen_data().export_png(file); } }
            if frame >= 12 { quit = true; }
        }
        if quit { if path.is_some() { records.observe(&game); persist(&records, path.as_ref()); } break; }
        next_frame().await;
    }
}
