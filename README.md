```text
████   ███  █   █  █████  █████  ████
█   █   █   ██  █  █      █      █   █
████    █   █ █ █  █  ██  ████   ████
█  █    █   █  ██  █   █  █      █  █
█   █  ███  █   █  █████  █████  █   █
```

A terminal countdown timer that warns you when you won't hear the alarm.

A timer is no use if you can't hear it. ringer notices when your computer is muted or too quiet, and tells you before you start, while it counts down, and when it rings.

![ringer counting down from 10 seconds with the volume warning showing](assets/demo.gif)

- Start it with a time, like `ringer 25m`, or type one in on screen
- Warns you if your sound is muted or too quiet to hear the alarm
- A `*~*~*~` wave scrolls across the screen while the timer runs
- The countdown fades from green to red as time runs out
- When time's up, an alarm sounds until you stop it, then the timer resets to the time you last used

## Install

Works on macOS (Apple Silicon and Intel) and Linux (x86_64 and arm64).

### macOS

With [Homebrew](https://brew.sh):

```sh
brew install gus-eisen/tap/ringer
```

Then start it from any terminal:

```sh
ringer
```

Update with `brew upgrade ringer`, uninstall with `brew uninstall ringer`.

#### Without Homebrew

Paste this into Terminal:

```sh
curl -fsSL https://github.com/Gus-Eisen/ringer/releases/latest/download/ringer-macos.tar.gz | tar xz \
  && sudo mkdir -p /usr/local/bin && sudo mv ringer /usr/local/bin/
```

It asks for your Mac password to install the program into `/usr/local/bin`.

> **Downloaded with a browser instead?** macOS blocks apps from unidentified
> developers that were downloaded in a browser. After extracting, run
> `xattr -d com.apple.quarantine ringer` once before moving it into place.

To uninstall: `sudo rm /usr/local/bin/ringer`

### Linux

Paste this into a terminal. It picks the right build for your machine and
asks for your password to install into `/usr/local/bin`:

```sh
curl -fsSL https://github.com/Gus-Eisen/ringer/releases/latest/download/ringer-linux-$(uname -m).tar.gz | tar xz \
  && sudo mkdir -p /usr/local/bin && sudo mv ringer /usr/local/bin/
```

The binary is static, so it runs on any distro. The volume warning reads your
default output from PipeWire (`wpctl`), PulseAudio (`pactl`) or ALSA
(`amixer`), and the alarm plays through `pw-play`, `paplay` or `aplay`,
whichever you have. Without any of them, the volume warning stays off and the
alarm rings the terminal bell instead.

To uninstall: `sudo rm /usr/local/bin/ringer`

### Build from source

Requires [Rust](https://rustup.rs) (2024 edition):

```sh
cargo install --git https://github.com/Gus-Eisen/ringer
```

## Usage

Start counting down right away by giving a time:

```sh
ringer 25m        # 25 minutes
ringer 1h30m      # 1 hour 30 minutes (also: ringer 1h 30m)
ringer 90s        # 90 seconds (a bare number like 90 also means seconds)
ringer 1:30       # minutes:seconds, or hours:minutes:seconds like 1:02:03
```

Or run `ringer` on its own and type digits to set the time. They fill in from the right, like a microwave:

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
static Linux binaries (x86_64 and arm64), publishes them as a release, and updates the Homebrew formula in
[Gus-Eisen/homebrew-tap](https://github.com/Gus-Eisen/homebrew-tap):

```sh
git tag v0.1.0 && git push origin v0.1.0
```

Bump `version` in `Cargo.toml` first for new versions.

## Project layout

```
src/
├── main.rs    # app state, key handling, drawing, animation
├── args.rs    # command-line times like 25m or 1:30
├── sound.rs   # mute / low-volume detection
├── alarm.rs   # looping alarm sound
└── font.rs    # big block digits for the countdown
```

The demo GIF is recorded with [vhs](https://github.com/charmbracelet/vhs) from `demo.tape`:
`cargo build --release && vhs demo.tape` (mute your Mac first so the volume warning shows).

Built with [ratatui](https://ratatui.rs) and [crossterm](https://github.com/crossterm-rs/crossterm).

## License

[MIT](LICENSE)
