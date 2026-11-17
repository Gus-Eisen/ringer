//! Plays an alarm sound on a loop until dropped.
//!
//! macOS plays a built-in system sound with `afplay`. Linux plays a chime
//! that ringer generates itself, through whichever of PipeWire (`pw-play`),
//! PulseAudio (`paplay`) or ALSA (`aplay`) is installed, so it doesn't depend
//! on a sound theme being present. If no player works, the terminal bell rings
//! instead.

use std::{
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub struct Alarm {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Alarm {
    pub fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let mut player = Player::new();
            while !flag.load(Ordering::Relaxed) {
                let Some(mut child) = player.play() else {
                    // No working player: fall back to the terminal bell.
                    print!("\x07");
                    thread::sleep(Duration::from_millis(800));
                    continue;
                };
                // Poll so the sound can be cut off immediately when stopped.
                loop {
                    if flag.load(Ordering::Relaxed) {
                        let _ = child.kill();
                        let _ = child.wait();
                        return;
                    }
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            // A player that's installed but can't reach the
                            // sound server exits with an error; try the next.
                            if !status.success() {
                                player.skip_current();
                            }
                            break;
                        }
                        Ok(None) => {}
                        Err(_) => {
                            player.skip_current();
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(30));
                }
            }
        });
        Self { stop, handle: Some(handle) }
    }
}

impl Drop for Alarm {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Tries each candidate command in turn, remembering the first that works.
struct Player {
    candidates: Vec<Vec<String>>,
    current: usize,
    /// A generated sound file to delete when the alarm stops.
    temp_file: Option<std::path::PathBuf>,
}

impl Drop for Player {
    fn drop(&mut self) {
        if let Some(path) = &self.temp_file {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl Player {
    fn play(&mut self) -> Option<Child> {
        while let Some(cmd) = self.candidates.get(self.current) {
            let spawned = Command::new(&cmd[0])
                .args(&cmd[1..])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            match spawned {
                Ok(child) => return Some(child),
                // Not installed: move on to the next candidate.
                Err(_) => self.current += 1,
            }
        }
        None
    }

    fn skip_current(&mut self) {
        self.current += 1;
    }
}

#[cfg(target_os = "macos")]
impl Player {
    fn new() -> Self {
        let cmd = |args: &[&str]| args.iter().map(|s| s.to_string()).collect();
        Self {
            candidates: vec![cmd(&["afplay", "/System/Library/Sounds/Glass.aiff"])],
            current: 0,
            temp_file: None,
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl Player {
    fn new() -> Self {
        let Some(path) = chime::write() else {
            return Self { candidates: Vec::new(), current: 0, temp_file: None };
        };
        let arg = path.to_string_lossy().into_owned();
        let cmd = |args: &[&str]| args.iter().map(|s| s.to_string()).collect();
        Self {
            candidates: vec![
                cmd(&["pw-play", &arg]),
                cmd(&["paplay", &arg]),
                cmd(&["aplay", "-q", &arg]),
            ],
            current: 0,
            temp_file: Some(path),
        }
    }
}

/// Generates the alarm chime as a WAV file.
#[cfg(any(not(target_os = "macos"), test))]
mod chime {
    use std::{f32::consts::TAU, path::PathBuf};

    const RATE: u32 = 44_100;
    const SECONDS: f32 = 1.2;
    /// C6, a bright bell-like pitch.
    const BASE_HZ: f32 = 1046.5;
    /// (frequency multiple, loudness, decay rate) for each partial.
    const PARTIALS: [(f32, f32, f32); 3] = [(1.0, 1.0, 3.0), (2.0, 0.4, 5.0), (3.0, 0.2, 8.0)];

    /// Writes the chime to the temp directory and returns its path.
    #[cfg_attr(test, allow(dead_code))]
    pub fn write() -> Option<PathBuf> {
        let path = std::env::temp_dir().join(format!("ringer-alarm-{}.wav", std::process::id()));
        std::fs::write(&path, wav()).ok()?;
        Some(path)
    }

    /// 16-bit mono PCM WAV bytes.
    pub fn wav() -> Vec<u8> {
        let samples = (RATE as f32 * SECONDS) as u32;
        let data_len = samples * 2;
        let mut out = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&1u16.to_le_bytes()); // mono
        out.extend_from_slice(&RATE.to_le_bytes());
        out.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
        out.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
        out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        let total: f32 = PARTIALS.iter().map(|p| p.1).sum();
        for i in 0..samples {
            let t = i as f32 / RATE as f32;
            // 5 ms fade-in avoids a click at the start.
            let attack = (t / 0.005).min(1.0);
            let v: f32 = PARTIALS
                .iter()
                .map(|&(mult, amp, decay)| amp * (-decay * t).exp() * (TAU * BASE_HZ * mult * t).sin())
                .sum();
            let sample = (v / total * attack * 0.8 * i16::MAX as f32) as i16;
            out.extend_from_slice(&sample.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn chime_is_a_valid_wav() {
        let wav = super::chime::wav();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        let riff_len = u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_len + 8, wav.len());
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(data_len + 44, wav.len());
    }
}
