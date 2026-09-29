#[allow(dead_code)]
#[path = "src/synth.rs"]
mod synth;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/synth.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var_os("CARGO_FEATURE_SOUND").is_none() { return Ok(()); }
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").ok_or("missing OUT_DIR")?);
    for i in 0..synth::TRACKS.len() {
        std::fs::write(out.join(format!("music-{i}.wav")), synth::music_wav(i))?;
    }
    for i in 0..synth::EFFECT_NAMES.len() {
        std::fs::write(out.join(format!("effect-{i}.wav")), synth::effect_wav(i))?;
    }
    Ok(())
}
