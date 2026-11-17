# ringer

A simple countdown timer for the terminal, written in Rust.

- Enter a time as `HH:MM:SS`
- A `*~*~*~` wave scrolls across the screen while the timer runs
- The countdown fades from green to red as time runs out
- When time's up, an alarm sounds until you stop it, then the timer resets to the time you last used
- Warns you if your sound is muted or the volume is too low to hear the alarm

## Install

Works on macOS (Apple Silicon and Intel). Paste this into Terminal:

```sh
curl -fsSL https://github.com/Gus-Eisen/ringer/releases/latest/download/ringer-macos.tar.gz | tar xz \
  && sudo mkdir -p /usr/local/bin && sudo mv ringer /usr/local/bin/
```

It asks for your Mac password to install the program into `/usr/local/bin`. Then start it from any terminal:

```sh
ringer
```

> **Downloaded with a browser instead?** macOS blocks apps from unidentified
> developers that were downloaded in a browser. After extracting, run
> `xattr -d com.apple.quarantine ringer` once before moving it into place.

To uninstall: `sudo rm /usr/local/bin/ringer`

### Build from source

Requires [Rust](https://rustup.rs) (2024 edition):

```sh
cargo install --git https://github.com/Gus-Eisen/ringer
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

A yellow banner appears above the clock, and the window border turns yellow, if:

- your sound is **muted**
- your volume is **0**
- your volume is **below 25%** (set by `LOW_VOLUME_THRESHOLD` in `src/sound.rs`)

The volume is checked when the app starts, every 2 seconds while it runs, and again when you start the timer. Unmute or turn the volume up and the banner disappears.

## Releasing

Push a version tag and GitHub Actions builds a universal macOS binary and
publishes it as a release:

```sh
git tag v0.1.0 && git push origin v0.1.0
```

Bump `version` in `Cargo.toml` first for new versions.

## Project layout

```
src/
├── main.rs    # app state, key handling, drawing, animation
├── sound.rs   # mute / low-volume detection
├── alarm.rs   # looping alarm sound
└── font.rs    # big block digits for the countdown
```

Built with [ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).

## License

[MIT](LICENSE)
