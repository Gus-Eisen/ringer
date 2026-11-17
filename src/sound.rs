//! Watches the output volume (macOS or Linux) in the background so the UI can warn
//! when the alarm would be inaudible.

use std::{
    process::Command,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Output volumes (percent) below this are considered too quiet to rely on.
/// 25% is 4 of the 16 steps on the keyboard volume keys.
const LOW_VOLUME_THRESHOLD: u32 = 25;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Muted,
    ZeroVolume,
    /// Volume is above 0 but below `LOW_VOLUME_THRESHOLD`; holds the percent.
    Low(u32),
    /// Couldn't determine the volume; don't show a warning.
    Unknown,
}

pub struct MuteMonitor {
    status: Arc<Mutex<Status>>,
    wake: Sender<()>,
}

impl MuteMonitor {
    /// Checks the volume once immediately, then every couple of seconds on a
    /// background thread.
    pub fn start() -> Self {
        let status = Arc::new(Mutex::new(query()));
        let (wake, rx): (Sender<()>, Receiver<()>) = mpsc::channel();
        let shared = Arc::clone(&status);
        thread::spawn(move || {
            loop {
                match rx.recv_timeout(POLL_INTERVAL) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {
                        *shared.lock().unwrap() = query();
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        });
        Self { status, wake }
    }

    /// Requests an immediate re-check (e.g. when the timer starts).
    pub fn check_now(&self) {
        let _ = self.wake.send(());
    }

    pub fn warning(&self) -> Option<String> {
        match *self.status.lock().unwrap() {
            Status::Muted => Some("Your sound is muted — you won't hear the alarm!".into()),
            Status::ZeroVolume => Some("Your volume is set to 0 — you won't hear the alarm!".into()),
            Status::Low(pct) => Some(format!("Your volume is very low ({pct}%) — you may not hear the alarm!")),
            Status::Ok | Status::Unknown => None,
        }
    }
}

/// Runs a command and returns its stdout if it succeeded.
fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Turns a mute flag and volume percent into a status.
fn classify(muted: bool, volume: Option<u32>) -> Status {
    if muted {
        return Status::Muted;
    }
    match volume {
        Some(0) => Status::ZeroVolume,
        Some(v) if v < LOW_VOLUME_THRESHOLD => Status::Low(v),
        Some(_) => Status::Ok,
        None => Status::Unknown,
    }
}

/// Asks macOS for the volume settings.
#[cfg(target_os = "macos")]
fn query() -> Status {
    run("osascript", &["-e", "get volume settings"]).map_or(Status::Unknown, |t| parse_osascript(&t))
}

/// Asks the Linux sound system about the default output, trying PipeWire,
/// then PulseAudio, then ALSA.
#[cfg(not(target_os = "macos"))]
fn query() -> Status {
    if let Some(t) = run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"])
        && let Some(status) = parse_wpctl(&t)
    {
        return status;
    }
    if let Some(mute) = run("pactl", &["get-sink-mute", "@DEFAULT_SINK@"])
        && let Some(vol) = run("pactl", &["get-sink-volume", "@DEFAULT_SINK@"])
        && let Some(status) = parse_pactl(&mute, &vol)
    {
        return status;
    }
    run("amixer", &["get", "Master"]).and_then(|t| parse_amixer(&t)).unwrap_or(Status::Unknown)
}

/// Parses osascript output like
/// `output volume:50, input volume:50, alert volume:100, output muted:false`
#[cfg(any(target_os = "macos", test))]
fn parse_osascript(text: &str) -> Status {
    let field = |name: &str| {
        text.split(',')
            .filter_map(|kv| kv.trim().split_once(':'))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.trim().to_string())
    };
    // "missing value" is reported for some outputs (e.g. HDMI) that can't be
    // controlled; that parses as no volume, so it's treated as unknown.
    classify(
        field("output muted").as_deref() == Some("true"),
        field("output volume").and_then(|v| v.parse::<u32>().ok()),
    )
}

/// Parses `wpctl get-volume` output like `Volume: 0.40` or
/// `Volume: 0.40 [MUTED]`. Volume can go above 1.00 (over 100%).
#[cfg(any(not(target_os = "macos"), test))]
fn parse_wpctl(text: &str) -> Option<Status> {
    let rest = text.trim().strip_prefix("Volume:")?;
    let level: f32 = rest.split_whitespace().next()?.parse().ok()?;
    let pct = (level * 100.0).round().max(0.0) as u32;
    Some(classify(rest.contains("[MUTED]"), Some(pct)))
}

/// Parses `pactl get-sink-mute` (`Mute: yes`) and `pactl get-sink-volume`
/// (`Volume: front-left: 26214 /  40% / -23.88 dB,   front-right: ...`)
/// output, using the loudest channel.
#[cfg(any(not(target_os = "macos"), test))]
fn parse_pactl(mute: &str, volume: &str) -> Option<Status> {
    let muted = match mute.trim().strip_prefix("Mute:")?.trim() {
        "yes" => true,
        "no" => false,
        _ => return None,
    };
    Some(classify(muted, Some(loudest_percent(volume)?)))
}

/// Parses `amixer get Master` output, whose channel lines look like
/// `  Front Left: Playback 26214 [40%] [-23.88dB] [on]` (`[off]` when muted).
#[cfg(any(not(target_os = "macos"), test))]
fn parse_amixer(text: &str) -> Option<Status> {
    let channels: Vec<&str> = text.lines().filter(|l| l.contains("Playback") && l.contains("%]")).collect();
    if channels.is_empty() {
        return None;
    }
    let muted = channels.iter().all(|l| l.contains("[off]"));
    Some(classify(muted, Some(loudest_percent(&channels.join(" "))?)))
}

/// The largest `NN%` value in the text.
#[cfg(any(not(target_os = "macos"), test))]
fn loudest_percent(text: &str) -> Option<u32> {
    text.split(|c: char| c.is_whitespace() || c == '[' || c == ']' || c == '/' || c == ',')
        .filter_map(|w| w.strip_suffix('%')?.parse::<u32>().ok())
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_osascript() {
        let s = |t| parse_osascript(t);
        assert!(s("output volume:50, input volume:50, alert volume:100, output muted:false") == Status::Ok);
        assert!(s("output volume:50, input volume:50, alert volume:100, output muted:true") == Status::Muted);
        assert!(s("output volume:0, input volume:50, alert volume:100, output muted:false") == Status::ZeroVolume);
        assert!(s("output volume:3, input volume:50, alert volume:100, output muted:false") == Status::Low(3));
        assert!(s("output volume:24, input volume:50, alert volume:100, output muted:false") == Status::Low(24));
        assert!(s("output volume:25, input volume:50, alert volume:100, output muted:false") == Status::Ok);
        assert!(s("output volume:missing value, input volume:50, alert volume:100, output muted:missing value") == Status::Unknown);
    }

    #[test]
    fn parses_wpctl() {
        assert!(parse_wpctl("Volume: 0.40\n") == Some(Status::Ok));
        assert!(parse_wpctl("Volume: 0.40 [MUTED]\n") == Some(Status::Muted));
        assert!(parse_wpctl("Volume: 0.00\n") == Some(Status::ZeroVolume));
        assert!(parse_wpctl("Volume: 0.10\n") == Some(Status::Low(10)));
        assert!(parse_wpctl("Volume: 1.50\n") == Some(Status::Ok));
        assert!(parse_wpctl("Could not connect to PipeWire\n").is_none());
    }

    #[test]
    fn parses_pactl() {
        let vol = |pct| format!(
            "Volume: front-left: 26214 /  {pct}% / -23.88 dB,   front-right: 26214 /  {pct}% / -23.88 dB\n        balance 0.00\n"
        );
        assert!(parse_pactl("Mute: no\n", &vol(40)) == Some(Status::Ok));
        assert!(parse_pactl("Mute: yes\n", &vol(40)) == Some(Status::Muted));
        assert!(parse_pactl("Mute: no\n", &vol(0)) == Some(Status::ZeroVolume));
        assert!(parse_pactl("Mute: no\n", &vol(12)) == Some(Status::Low(12)));
        assert!(parse_pactl("garbage", &vol(40)).is_none());
    }

    #[test]
    fn parses_amixer() {
        let text = |pct, sw| format!(
            "Simple mixer control 'Master',0\n  Capabilities: pvolume pswitch\n  Playback channels: Front Left - Front Right\n  Limits: Playback 0 - 65536\n  Mono:\n  Front Left: Playback 26214 [{pct}%] [{sw}]\n  Front Right: Playback 26214 [{pct}%] [{sw}]\n"
        );
        assert!(parse_amixer(&text(40, "on")) == Some(Status::Ok));
        assert!(parse_amixer(&text(40, "off")) == Some(Status::Muted));
        assert!(parse_amixer(&text(0, "on")) == Some(Status::ZeroVolume));
        assert!(parse_amixer(&text(20, "on")) == Some(Status::Low(20)));
        assert!(parse_amixer("amixer: Unable to find simple control 'Master',0\n").is_none());
    }
}
