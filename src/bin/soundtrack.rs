use std::path::PathBuf;
use tetris_core::synth;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("soundtrack"));
    std::fs::create_dir_all(&dir)?;
    for (i, track) in synth::TRACKS.iter().enumerate() {
        let path = dir.join(format!("{:02}-{}.wav", i + 1, track.name.replace(' ', "-")));
        std::fs::write(&path, synth::music_wav(i))?;
        println!("{} | {} BPM", path.display(), track.bpm);
    }
    Ok(())
}
