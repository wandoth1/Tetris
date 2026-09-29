//! Original music and effects: no MIDI driver, soundfonts, samples or network.
//! PCM WAVs are synthesized at BUILD time and embedded into the executable.
use std::f32::consts::TAU;
pub const SAMPLE_RATE: u32 = 22_050;
#[derive(Clone, Copy)]
pub struct Track { pub name: &'static str, pub bpm: f32, pub root: i32 }
pub const TRACKS: [Track; 3] = [
    Track { name: "Neon Descent", bpm: 120.0, root: 45 },
    Track { name: "Night Circuit", bpm: 132.0, root: 47 },
    Track { name: "Afterglow", bpm: 96.0, root: 42 },
];
pub const EFFECT_NAMES: [&str; 8] = ["move", "rotate", "drop", "clear", "quad", "hold", "over", "win"];
fn frequency(note: i32) -> f32 { 440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0) }
fn voice(note: i32, time: f32, length: f32, bright: bool) -> f32 {
    if time < 0.0 || time >= length { return 0.0; }
    let phase = TAU * frequency(note) * time;
    let attack = (time / 0.006).min(1.0); let release = ((length - time) / 0.025).clamp(0.0, 1.0);
    let wave = if bright { phase.sin() + 0.26 * (3.0 * phase).sin() + 0.09 * (5.0 * phase).sin() }
        else { phase.sin() + 0.18 * (2.0 * phase).sin() };
    wave * attack * release * (-2.6 * time / length).exp()
}
fn noise(sample: u32) -> f32 {
    let mut n = sample.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277_803_737); n = (n >> 22) ^ n;
    (n as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32
}
fn kick(t: f32) -> f32 {
    if t > 0.30 { return 0.0; }
    let phase = TAU * (45.0 * t + 115.0 * (1.0 - (-32.0 * t).exp()) / 32.0);
    phase.sin() * (-17.0 * t).exp() * (t / 0.002).min(1.0)
}
fn snare(t: f32, sample: u32) -> f32 {
    if t > 0.20 { return 0.0; }
    (noise(sample) * 0.7 + (TAU * 175.0 * t).sin() * 0.3) * (-24.0 * t).exp() * (t / 0.001).min(1.0)
}
fn hat(t: f32, sample: u32, open: bool) -> f32 {
    let length = if open { 0.12 } else { 0.045 }; if t >= length { return 0.0; }
    let high = (noise(sample) - noise(sample.wrapping_sub(1))) * 0.5;
    high * (-7.0 * t / length).exp() * (t / 0.001).min(1.0)
}
/// Sixteen bars with original chord-relative melodic phrases.
pub fn music_wav(index: usize) -> Vec<u8> {
    let index = index % TRACKS.len(); let track = TRACKS[index]; let beat = 60.0 / track.bpm;
    let count = (64.0 * beat * SAMPLE_RATE as f32).round() as usize;
    let mut dry = Vec::with_capacity(count);
    let phrases: [[i32; 16]; 3] = [
        [12, 7, 15, 19, 14, 12, 7, 10, 12, 19, 22, 19, 15, 14, 10, 7],
        [7, 12, 19, 15, 14, 19, 22, 24, 19, 15, 12, 10, 7, 10, 14, 19],
        [12, 19, 15, 10, 7, 14, 12, 7, 10, 15, 19, 22, 19, 14, 12, 10],
    ];
    let harmony = [0, -4, 3, -2];
    for i in 0..count {
        let t = i as f32 / SAMPLE_RATE as f32; let b = t / beat;
        let bar = (b / 4.0).floor() as usize; let step = (b * 2.0).floor() as usize; let quarter = b.floor() as usize;
        let root = track.root + harmony[bar % 4]; let third = if bar % 4 == 0 { 3 } else { 4 };
        let chord = [0, third, 7, 12, 7, third, 12, 7];
        let eighth_time = (b * 2.0).fract() * beat / 2.0; let quarter_time = b.fract() * beat;
        let mut value = 0.0;
        value += 0.25 * voice(root - 12 + if step % 4 == 3 { 12 } else { 0 }, eighth_time, beat * 0.47, false);
        value += 0.105 * voice(root + 12 + chord[step % 8], eighth_time, beat * 0.46, true);
        if quarter % 8 != 7 {
            let note = root + phrases[index][quarter % 16] + if bar >= 12 { 12 } else { 0 };
            value += 0.15 * voice(note, quarter_time, beat * 0.82, true);
        }
        let bar_time = (b % 4.0) * beat;
        for interval in [0, third, 7] { value += 0.043 * voice(root + 12 + interval, bar_time, beat * 4.0, false); }
        value += 0.46 * kick((b % 2.0) * beat);
        if quarter % 2 == 1 { value += 0.22 * snare(quarter_time, i as u32); }
        value += 0.065 * hat(eighth_time, i as u32, step % 8 == 7); dry.push(value);
    }
    // Circular delays keep echo alive across the loop boundary.
    let delay_l = (beat * 0.75 * SAMPLE_RATE as f32) as usize; let delay_r = (beat * 1.25 * SAMPLE_RATE as f32) as usize;
    let frames = (0..count).map(|i| {
        let l = dry[i] + 0.22 * dry[(i + count - delay_l) % count]; let r = dry[i] + 0.22 * dry[(i + count - delay_r) % count];
        (0.83 * (l * 1.1).tanh(), 0.83 * (r * 1.1).tanh())
    }); wav(frames, count)
}
pub fn effect_wav(index: usize) -> Vec<u8> {
    let duration = match index { 0 => 0.04, 1 => 0.075, 2 => 0.16, 3 => 0.32, 4 => 0.56, 5 => 0.13, 6 => 0.90, _ => 1.20 };
    let count = (duration * SAMPLE_RATE as f32) as usize;
    let frames = (0..count).map(|i| {
        let t = i as f32 / SAMPLE_RATE as f32;
        let v = match index {
            0 => voice(76, t, duration, false) * 0.25,
            1 => voice(83, t, duration, true) * 0.25,
            2 => kick(t) * 0.6 + noise(i as u32) * (-45.0 * t).exp() * 0.09,
            3 | 4 | 7 => {
                let notes: &[i32] = if index == 3 { &[72, 76, 79, 84] } else if index == 4 { &[72, 76, 79, 84, 88, 91] } else { &[72, 76, 79, 84, 79, 84, 88, 91] };
                let slice = duration / notes.len() as f32; let step = ((t / slice) as usize).min(notes.len() - 1);
                voice(notes[step], t % slice, slice, true) * 0.42
            }
            5 => voice(67, t, duration, false) * 0.25 + voice(79, t, duration, false) * 0.2,
            _ => { let notes = [60, 55, 51, 48, 43, 36]; let slice = duration / notes.len() as f32;
                voice(notes[((t / slice) as usize).min(5)], t % slice, slice, false) * 0.44 }
        };
        let edge = (t / 0.002).min(1.0) * ((duration - t) / 0.01).clamp(0.0, 1.0);
        let v = (v * edge).clamp(-0.9, 0.9); (v, v)
    }); wav(frames, count)
}
fn wav(frames: impl Iterator<Item = (f32, f32)>, count: usize) -> Vec<u8> {
    let data_size = (count * 4) as u32; let mut out = Vec::with_capacity(44 + data_size as usize);
    out.extend_from_slice(b"RIFF"); out.extend_from_slice(&(36 + data_size).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt "); out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&1_u16.to_le_bytes()); out.extend_from_slice(&2_u16.to_le_bytes());
    out.extend_from_slice(&SAMPLE_RATE.to_le_bytes()); out.extend_from_slice(&(SAMPLE_RATE * 4).to_le_bytes());
    out.extend_from_slice(&4_u16.to_le_bytes()); out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(b"data"); out.extend_from_slice(&data_size.to_le_bytes());
    for (left, right) in frames { for sample in [left, right] {
        let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16; out.extend_from_slice(&pcm.to_le_bytes());
    } } out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn a4_is_440_hz() { assert!((frequency(69) - 440.0).abs() < 0.001); }
    #[test] fn octave_doubles_frequency() { assert!((frequency(81) / frequency(69) - 2.0).abs() < 0.001); }
    #[test] fn voices_have_finite_envelopes() { assert_eq!(voice(60, -0.1, 1.0, true), 0.0); assert_eq!(voice(60, 1.1, 1.0, true), 0.0); assert_eq!(voice(60, 0.0, 1.0, true), 0.0); }
    #[test] fn effects_have_valid_wav_headers() { for i in 0..8 { let bytes = effect_wav(i); assert_eq!(&bytes[..4], b"RIFF"); assert_eq!(&bytes[8..12], b"WAVE"); let n = u32::from_le_bytes(bytes[40..44].try_into().unwrap()); assert_eq!(bytes.len(), n as usize + 44); } }
    #[test] fn effects_are_stereo_16_bit() { let b = effect_wav(0); assert_eq!(u16::from_le_bytes([b[22], b[23]]), 2); assert_eq!(u16::from_le_bytes([b[34], b[35]]), 16); }
    #[test] fn effects_are_deterministic() { assert_eq!(effect_wav(2), effect_wav(2)); }
    #[test] fn noise_stays_bounded() { for i in 0..10000 { assert!((-1.0..=1.0).contains(&noise(i))); } }
    #[test] fn music_has_expected_duration_and_no_clipped_samples() {
        let b = music_wav(0); assert_eq!(b.len(), 44 + 32 * SAMPLE_RATE as usize * 4);
        let peak = b[44..].chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]]).unsigned_abs()).max().unwrap();
        assert!(peak > 1000 && peak < 31000);
    }
}
