use tetris_core::storage::Records;
#[cfg(feature = "sound")]
mod enabled {
    use super::*;
    use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
    pub struct Audio {
        music: Vec<Option<Sound>>, effects: Vec<Option<Sound>>, playing: Option<usize>, pub available: bool,
    }
    impl Audio {
        pub async fn load() -> Self {
            let music_bytes: [&[u8]; 3] = [
                include_bytes!(concat!(env!("OUT_DIR"), "/music-0.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/music-1.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/music-2.wav")),
            ];
            let effect_bytes: [&[u8]; 8] = [
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-0.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-1.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-2.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-3.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-4.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-5.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-6.wav")),
                include_bytes!(concat!(env!("OUT_DIR"), "/effect-7.wav")),
            ];
            let mut music = Vec::new(); let mut effects = Vec::new();
            for bytes in music_bytes { music.push(load_sound_from_bytes(bytes).await.ok()); }
            for bytes in effect_bytes { effects.push(load_sound_from_bytes(bytes).await.ok()); }
            let available = music.iter().all(Option::is_some) && effects.iter().all(Option::is_some);
            Self { music, effects, playing: None, available }
        }
        pub fn sync(&mut self, settings: &Records, quiet: bool) {
            let requested = settings.music.then_some(settings.track % self.music.len());
            if requested != self.playing {
                if let Some(i) = self.playing { if let Some(sound) = &self.music[i] { stop_sound(sound); } }
                if let Some(i) = requested { if let Some(sound) = &self.music[i] { play_sound(sound, PlaySoundParams { looped: true, volume: 0.0 }); } }
                self.playing = requested;
            }
            if let Some(i) = self.playing { if let Some(sound) = &self.music[i] { set_sound_volume(sound, settings.volume * if quiet { 0.2 } else { 0.65 }); } }
        }
        pub fn effect(&self, index: usize, settings: &Records) {
            if !settings.effects { return; }
            if let Some(Some(sound)) = self.effects.get(index) {
                stop_sound(sound); play_sound(sound, PlaySoundParams { looped: false, volume: settings.volume * 0.85 });
            }
        }
    }
}
#[cfg(feature = "sound")]
pub use enabled::Audio;
#[cfg(not(feature = "sound"))]
pub struct Audio { pub available: bool }
#[cfg(not(feature = "sound"))]
impl Audio {
    pub async fn load() -> Self { Self { available: false } }
    pub fn sync(&mut self, _: &Records, _: bool) {}
    pub fn effect(&self, _: usize, _: &Records) {}
}
