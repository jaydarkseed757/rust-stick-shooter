use rodio::{OutputStream, OutputStreamHandle, Source};
use std::collections::VecDeque;
use std::time::Duration;

// ── Waveform types ────────────────────────────────────────────────────────────

#[derive(Clone)]
enum WaveKind { Square, Noise }

// ── A single note segment ─────────────────────────────────────────────────────

#[derive(Clone)]
struct Note {
    freq_start: f32,
    freq_end:   f32,
    total:      u32,   // samples
    volume:     f32,
    kind:       WaveKind,
}

// ── Multi-note chip source ────────────────────────────────────────────────────
//
// Plays a queue of notes in sequence. Each note is a square-wave or LFSR noise
// segment with optional linear frequency sweep and a short fade-out envelope.

pub struct ChipSource {
    notes: VecDeque<Note>,
    pos:   u32,    // sample position within the current note
    lfsr:  u16,    // Galois LFSR state for noise channel
}

impl ChipSource {
    const SR: u32 = 44_100;

    // Single square-wave note with optional frequency sweep.
    pub fn square(freq_start: f32, freq_end: f32, ms: u32, vol: f32) -> Self {
        Self::new_notes(vec![Note {
            freq_start, freq_end,
            total: Self::SR * ms / 1000,
            volume: vol, kind: WaveKind::Square,
        }])
    }

    // Single noise burst.
    pub fn noise(ms: u32, vol: f32) -> Self {
        Self::new_notes(vec![Note {
            freq_start: 0.0, freq_end: 0.0,
            total: Self::SR * ms / 1000,
            volume: vol, kind: WaveKind::Noise,
        }])
    }

    // Sequence of square-wave notes: &[(freq_start, freq_end, ms, vol)]
    pub fn sequence(notes: &[(f32, f32, u32, f32)]) -> Self {
        Self::new_notes(notes.iter().map(|&(fs, fe, ms, v)| Note {
            freq_start: fs, freq_end: fe,
            total: Self::SR * ms / 1000,
            volume: v, kind: WaveKind::Square,
        }).collect())
    }

    fn new_notes(notes: Vec<Note>) -> Self {
        ChipSource { notes: VecDeque::from(notes), pos: 0, lfsr: 0xACE1 }
    }
}

impl Iterator for ChipSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        loop {
            let note = self.notes.front()?;
            if self.pos >= note.total {
                self.notes.pop_front();
                self.pos = 0;
                continue;
            }

            let t   = self.pos as f32 / note.total as f32;
            // Short linear fade-out over the last 15% of each note.
            let env = if t > 0.85 { (1.0 - t) / 0.15 } else { 1.0 };

            let raw = match note.kind {
                WaveKind::Square => {
                    let freq   = note.freq_start + (note.freq_end - note.freq_start) * t;
                    let period = Self::SR as f32 / freq.max(1.0);
                    if (self.pos as f32 % period) < period * 0.5 { 1.0f32 } else { -1.0 }
                }
                WaveKind::Noise => {
                    // Galois 16-bit LFSR — classic chiptune noise generator.
                    let bit = self.lfsr & 1;
                    self.lfsr >>= 1;
                    if bit != 0 { self.lfsr ^= 0xB400; }
                    if bit != 0 { 1.0f32 } else { -1.0 }
                }
            };

            self.pos += 1;
            return Some(raw * note.volume * env);
        }
    }
}

impl Source for ChipSource {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self)                            -> u16  { 1 }
    fn sample_rate(&self)                         -> u32  { Self::SR }
    fn total_duration(&self) -> Option<Duration>          { None }
}

// ── Two-channel mixer ─────────────────────────────────────────────────────────
//
// Sums two ChipSources sample-by-sample. Used for noise + tone explosions.

struct Mix2 { a: ChipSource, b: ChipSource }

impl Iterator for Mix2 {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        let sa = self.a.next();
        let sb = self.b.next();
        match (sa, sb) {
            (None,    None)    => None,
            (Some(x), None)    => Some(x),
            (None,    Some(y)) => Some(y),
            (Some(x), Some(y)) => Some((x + y).clamp(-1.0, 1.0)),
        }
    }
}

impl Source for Mix2 {
    fn current_frame_len(&self) -> Option<usize> { None }
    fn channels(&self)                            -> u16  { 1 }
    fn sample_rate(&self)                         -> u32  { ChipSource::SR }
    fn total_duration(&self) -> Option<Duration>          { None }
}

// ── Sound effect catalogue ────────────────────────────────────────────────────

pub enum Sfx {
    Shoot,       // player fires a bullet
    KillSmall,   // grunt dies
    KillBig,     // spheroid / tank / enforcer / phantom / bomber dies
    PlayerHit,   // player loses a life (non-fatal)
    WaveStart,   // new wave of enemies spawns
    WaveClear,   // all enemies destroyed
    GameOver,    // player loses last life
}

// ── Sound system ──────────────────────────────────────────────────────────────

pub struct SoundSystem {
    _stream: OutputStream,           // must stay alive — owns the audio device
    handle:  OutputStreamHandle,
}

impl SoundSystem {
    /// Returns None gracefully if no audio device is available.
    pub fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(SoundSystem { _stream: stream, handle })
    }

    pub fn play(&self, sfx: Sfx) {
        // play_raw feeds directly into rodio's mixer, which resamples to the
        // device rate and up-mixes mono → stereo automatically.
        let _ = match sfx {
            Sfx::Shoot =>
                self.handle.play_raw(
                    ChipSource::square(180.0, 720.0, 55, 0.16)
                ),
            Sfx::KillSmall =>
                self.handle.play_raw(
                    ChipSource::noise(88, 0.28)
                ),
            Sfx::KillBig =>
                self.handle.play_raw(Mix2 {
                    a: ChipSource::noise(220, 0.26),
                    b: ChipSource::square(90.0, 28.0, 220, 0.20),
                }),
            Sfx::PlayerHit =>
                self.handle.play_raw(
                    ChipSource::square(700.0, 42.0, 480, 0.38)
                ),
            Sfx::WaveStart =>
                self.handle.play_raw(
                    ChipSource::sequence(&[
                        (330.0, 330.0,  80, 0.28),
                        (440.0, 440.0,  80, 0.28),
                        (660.0, 660.0, 130, 0.28),
                    ])
                ),
            Sfx::WaveClear =>
                self.handle.play_raw(
                    ChipSource::sequence(&[
                        (440.0, 440.0,  70, 0.24),
                        (550.0, 550.0,  70, 0.24),
                        (880.0, 880.0, 110, 0.24),
                    ])
                ),
            Sfx::GameOver =>
                self.handle.play_raw(
                    ChipSource::sequence(&[
                        (440.0, 440.0, 160, 0.34),
                        (330.0, 330.0, 160, 0.34),
                        (220.0,  90.0, 520, 0.34),
                    ])
                ),
        };
    }
}
