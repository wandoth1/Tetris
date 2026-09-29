//! Small bounded, versioned local save file. No personal data or telemetry.
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use crate::engine::{Game, Mode};

#[derive(Clone, Debug, PartialEq)]
pub struct Records {
    pub best: [u64; 3], pub sprint_seconds: Option<f64>, pub music: bool, pub effects: bool,
    pub reduced_motion: bool, pub volume: f32, pub track: usize,
}
impl Default for Records {
    fn default() -> Self { Self { best: [0; 3], sprint_seconds: None, music: true, effects: true, reduced_motion: false, volume: 0.45, track: 0 } }
}
impl Records {
    pub fn parse(text: &str) -> Self {
        let mut r = Self::default();
        if text.len() > 65_536 || !text.lines().any(|line| line.trim() == "version=1") { return r; }
        for line in text.lines() {
            let Some((key, value)) = line.trim().split_once('=') else { continue; };
            match key {
                "best0" => { if let Ok(v) = value.parse() { r.best[0] = v; } }
                "best1" => { if let Ok(v) = value.parse() { r.best[1] = v; } }
                "best2" => { if let Ok(v) = value.parse() { r.best[2] = v; } }
                "sprint_seconds" => { if let Ok(v) = value.parse::<f64>() { if v.is_finite() && v > 0.0 { r.sprint_seconds = Some(v); } } }
                "music" => { if let Ok(v) = value.parse() { r.music = v; } }
                "effects" => { if let Ok(v) = value.parse() { r.effects = v; } }
                "reduced_motion" => { if let Ok(v) = value.parse() { r.reduced_motion = v; } }
                "volume" => { if let Ok(v) = value.parse::<f32>() { if v.is_finite() { r.volume = v.clamp(0.0, 1.0); } } }
                "track" => { if let Ok(v) = value.parse::<usize>() { r.track = v % 3; } }
                _ => {}
            }
        } r
    }
    pub fn encode(&self) -> String {
        format!("version=1\nbest0={}\nbest1={}\nbest2={}\nsprint_seconds={}\nmusic={}\neffects={}\nreduced_motion={}\nvolume={}\ntrack={}\n",
            self.best[0], self.best[1], self.best[2], self.sprint_seconds.unwrap_or(0.0), self.music,
            self.effects, self.reduced_motion, self.volume, self.track)
    }
    pub fn observe(&mut self, game: &Game) {
        self.best[game.mode.index()] = self.best[game.mode.index()].max(game.score);
        if game.mode == Mode::Sprint && game.won && game.elapsed > 0.0 && self.sprint_seconds.is_none_or(|v| game.elapsed < v) { self.sprint_seconds = Some(game.elapsed); }
    }
    pub fn load(path: &Path) -> Self {
        let Ok(file) = File::open(path) else { return Self::default(); }; let mut text = String::new();
        match file.take(65_537).read_to_string(&mut text) { Ok(_) => Self::parse(&text), Err(_) => Self::default() }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() { if !parent.as_os_str().is_empty() { fs::create_dir_all(parent)?; } }
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        let result = (|| {
            let mut file = File::create(&tmp)?; file.write_all(self.encode().as_bytes())?;
            file.sync_all()?; drop(file); fs::rename(&tmp, path)
        })();
        if result.is_err() { let _ = fs::remove_file(&tmp); } result
    }
}
pub fn save_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    base.filter(|p| p.is_absolute()).map(|p| p.join("TetrisNeonPulse").join("records-v1.ini"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn missing_data_defaults() { assert_eq!(Records::parse(""), Records::default()); }
    #[test] fn rejects_unknown_version() { assert_eq!(Records::parse("version=99\nbest0=999"), Records::default()); }
    #[test] fn round_trip() { let r = Records { best: [100, 200, 300], sprint_seconds: Some(70.25), ..Records::default() }; assert_eq!(Records::parse(&r.encode()), r); }
    #[test] fn corrupt_values_are_safe() { let r = Records::parse("version=1\nvolume=NaN\nsprint_seconds=inf\ntrack=900\nbest0=-50\n"); assert_eq!(r.volume, 0.45); assert_eq!(r.sprint_seconds, None); assert_eq!(r.track, 0); assert_eq!(r.best[0], 0); }
    #[test] fn volume_clamps() { assert_eq!(Records::parse("version=1\nvolume=9").volume, 1.0); assert_eq!(Records::parse("version=1\nvolume=-9").volume, 0.0); }
    #[test] fn oversized_file_is_ignored() { assert_eq!(Records::parse(&format!("version=1\n{}", "a".repeat(70_000))), Records::default()); }
    #[test] fn records_do_not_decrease() { let mut r = Records::default(); let mut g = Game::new(Mode::Marathon, 1); g.score = 500; r.observe(&g); g.score = 100; r.observe(&g); assert_eq!(r.best[0], 500); }
    #[test] fn unfinished_sprint_is_not_recorded() { let mut r = Records::default(); let mut g = Game::new(Mode::Sprint, 1); g.elapsed = 10.0; r.observe(&g); assert_eq!(r.sprint_seconds, None); }
    #[test] fn shorter_completed_sprint_wins() { let mut r = Records::default(); let mut g = Game::new(Mode::Sprint, 1); g.elapsed = 80.0; g.won = true; r.observe(&g); g.elapsed = 70.0; r.observe(&g); g.elapsed = 90.0; r.observe(&g); assert_eq!(r.sprint_seconds, Some(70.0)); }
    #[test] fn saves_and_replaces_file() {
        let p = std::env::temp_dir().join(format!("tetris-test-{}-records.ini", std::process::id()));
        let mut r = Records::default(); r.save(&p).unwrap(); r.best[0] = 789; r.save(&p).unwrap();
        assert_eq!(Records::load(&p).best[0], 789); fs::remove_file(p).unwrap();
    }
}
