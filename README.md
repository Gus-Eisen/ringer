# rust-timer

A simple countdown timer for the terminal, written in Rust.

- Enter a time as `HH:MM:SS`
- A `*~*~*~` wave scrolls across the screen while the timer runs
- When time's up, an alarm sounds until you stop it, then the timer resets to the time you last used
- Warns you if your sound is muted or the volume is too low to hear the alarm

## Requirements

- **macOS**: the alarm uses `afplay` and the mute check uses `osascript`, both built into macOS
- **Rust** (2024 edition): install from [rustup.rs](https://rustup.rs)

## Install & run

```sh
git clone https://github.com/Gus-Eisen/rust-timer.git
cd rust-timer
cargo run --release
```

Or install it so you can run `rust-timer` from anywhere:

```sh
cargo install --path .
```

## Usage

Type digits to set the time. They fill in from the right, like a microwave:

| You type | Timer is set to |
|----------|-----------------|
| `30`     | `00:00:30`      |
| `130`    | `00:01:30`      |
| `10000`  | `01:00:00`      |

### Controls

| Screen         | Key                       | Action              |
|----------------|---------------------------|---------------------|
| Setting time   | `0`–`9`                   | Enter digits        |
|                | `Backspace`               | Erase last digit    |
|                | `Delete`                  | Clear the time      |
|                | `Enter`                   | Start the timer     |
| Timer running  | `Esc`                     | Cancel              |
| Alarm ringing  | `Enter` / `Space` / `Esc` | Stop the alarm      |
| Anywhere       | `q` / `Ctrl+C`            | Quit                |

### Volume warning

A yellow banner appears at the top of the screen if:

- your sound is **muted**
- your volume is **0**
- your volume is **below 25%** (set by `LOW_VOLUME_THRESHOLD` in `src/sound.rs`)

The volume is checked when the app starts, every 2 seconds while it runs, and again when you start the timer. Unmute or turn the volume up and the banner disappears.

## Project layout

```
src/
├── main.rs    # app state, key handling, drawing, animation
├── sound.rs   # mute / low-volume detection
├── alarm.rs   # looping alarm sound
└── font.rs    # big block digits for the countdown
```

Built with [ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).
