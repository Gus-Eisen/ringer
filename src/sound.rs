//! Watches the macOS output volume in the background so the UI can warn
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

/// Asks macOS for the volume settings. Output looks like:
/// `output volume:50, input volume:50, alert volume:100, output muted:false`
fn query() -> Status {
    let Ok(out) = Command::new("osascript")
        .args(["-e", "get volume settings"])
        .output()
    else {
        return Status::Unknown;
    };
    if !out.status.success() {
        return Status::Unknown;
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

fn parse(text: &str) -> Status {
    let field = |name: &str| {
        text.split(',')
            .filter_map(|kv| kv.trim().split_once(':'))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.trim().to_string())
    };
    if field("output muted").as_deref() == Some("true") {
        return Status::Muted;
    }
    match field("output volume").and_then(|v| v.parse::<u32>().ok()) {
        Some(0) => Status::ZeroVolume,
        Some(v) if v < LOW_VOLUME_THRESHOLD => Status::Low(v),
        Some(_) => Status::Ok,
        // "missing value" is reported for some outputs (e.g. HDMI) that
        // can't be controlled; treat as unknown.
        None => Status::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_volume_settings() {
        let s = |t| parse(t);
        assert!(s("output volume:50, input volume:50, alert volume:100, output muted:false") == Status::Ok);
        assert!(s("output volume:50, input volume:50, alert volume:100, output muted:true") == Status::Muted);
        assert!(s("output volume:0, input volume:50, alert volume:100, output muted:false") == Status::ZeroVolume);
        assert!(s("output volume:3, input volume:50, alert volume:100, output muted:false") == Status::Low(3));
        assert!(s("output volume:24, input volume:50, alert volume:100, output muted:false") == Status::Low(24));
        assert!(s("output volume:25, input volume:50, alert volume:100, output muted:false") == Status::Ok);
        assert!(s("output volume:missing value, input volume:50, alert volume:100, output muted:missing value") == Status::Unknown);
    }
}
