//! Original synthesized sound effects. No compressed audio decoders or external
//! assets are used. Sample generation is identical to the native game; playback
//! uses the Web Audio API.
pub const SAMPLE_RATE: u32 = 44100;

#[derive(Clone, Copy)]
pub enum Sound {
    Fire,
    Pistol,
    Sniper,
    Reload,
    Hit,
    Head,
    Kill,
    Footstep,
    Land,
}

/// `(frequency, duration, noise)` for each [`Sound`], indexed by `sound as usize`.
const PARAMS: [(f32, f32, f32); 9] = [
    (110., 0.075, 0.7),
    (240., 0.06, 0.65),
    (75., 0.18, 0.8),
    (350., 0.13, 0.4),
    (800., 0.05, 0.0),
    (1400., 0.07, 0.0),
    (520., 0.16, 0.0),
    (92., 0.085, 0.72),
    (58., 0.16, 0.78),
];

fn synthesize(frequency: f32, duration: f32, noise: f32) -> Vec<f32> {
    let count = (duration * SAMPLE_RATE as f32) as usize;
    let mut seed = 17u32;
    (0..count)
        .map(|i| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope = (1. - i as f32 / count as f32).powi(3) * (t / 0.002).min(1.);
            let random = (seed >> 8) as f32 / 8388608. - 1.;
            let tone = (std::f32::consts::TAU * frequency * t).sin();
            0.18 * envelope * (tone * (1. - noise) + random * noise)
        })
        .collect()
}

/// Generates every effect in [`Sound`] order.
pub fn synthesize_all() -> Vec<Vec<f32>> {
    PARAMS
        .into_iter()
        .map(|(frequency, duration, noise)| synthesize(frequency, duration, noise))
        .collect()
}

#[cfg(target_arch = "wasm32")]
pub struct Audio {
    context: web_sys::AudioContext,
    gain: web_sys::GainNode,
    buffers: Vec<web_sys::AudioBuffer>,
    volume: f32,
}

#[cfg(target_arch = "wasm32")]
impl Audio {
    pub fn new() -> Result<Self, String> {
        let context =
            web_sys::AudioContext::new().map_err(|_| "AudioContext を作成できません".to_owned())?;
        let gain = context
            .create_gain()
            .map_err(|_| "GainNode を作成できません".to_owned())?;
        gain.connect_with_audio_node(&context.destination())
            .map_err(|_| "音声出力を接続できません".to_owned())?;
        gain.gain().set_value(1.0);
        let buffers = synthesize_all()
            .into_iter()
            .map(|samples| {
                let buffer = context
                    .create_buffer(1, samples.len() as u32, SAMPLE_RATE as f32)
                    .map_err(|_| "AudioBuffer を作成できません".to_owned())?;
                buffer
                    .copy_to_channel(&samples, 0)
                    .map_err(|_| "AudioBuffer へ書き込めません".to_owned())?;
                Ok(buffer)
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            context,
            gain,
            buffers,
            volume: 1.0,
        })
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = if volume.is_finite() {
            volume.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.gain.gain().set_value(self.volume);
    }

    pub fn play(&self, sound: Sound) {
        if self.context.state() == web_sys::AudioContextState::Suspended {
            let _ = self.context.resume();
        }
        let Ok(source) = self.context.create_buffer_source() else {
            return;
        };
        source.set_buffer(Some(&self.buffers[sound as usize]));
        if source.connect_with_audio_node(&self.gain).is_err() {
            return;
        }
        let _ = source.start();
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::{synthesize_all, Sound, SAMPLE_RATE};

    #[test]
    fn every_effect_is_generated_with_the_expected_length() {
        let sounds = synthesize_all();
        assert_eq!(sounds.len(), 9);
        assert_eq!(
            sounds[Sound::Fire as usize].len(),
            (0.075 * SAMPLE_RATE as f32) as usize
        );
        assert_eq!(
            sounds[Sound::Sniper as usize].len(),
            (0.18 * SAMPLE_RATE as f32) as usize
        );
    }

    #[test]
    fn synthesis_is_deterministic_and_in_range() {
        let first = synthesize_all();
        let second = synthesize_all();
        assert_eq!(first, second);
        assert!(first
            .iter()
            .flatten()
            .all(|sample| sample.is_finite() && sample.abs() <= 0.18));
    }
}
