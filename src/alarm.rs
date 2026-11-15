//! Plays a system sound on a loop until dropped.

use std::{
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

const SOUND: &str = "/System/Library/Sounds/Glass.aiff";

pub struct Alarm {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Alarm {
    pub fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                let Ok(mut child) = play() else {
                    // afplay unavailable: fall back to the terminal bell.
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
                    if let Ok(Some(_)) = child.try_wait() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(30));
                }
            }
        });
        Self { stop, handle: Some(handle) }
    }
}

fn play() -> std::io::Result<Child> {
    Command::new("afplay")
        .arg(SOUND)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
}

impl Drop for Alarm {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
