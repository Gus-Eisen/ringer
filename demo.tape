# Records the README demo GIF. Run from the repo root:
#
#   cargo build --release && vhs demo.tape
#
# Mute your Mac first so the volume warning shows and the alarm stays quiet.

Output assets/demo.gif

Set Shell zsh
Set FontSize 18
Set Width 1000
Set Height 540
Set Padding 16
Set Framerate 24
Set TypingSpeed 180ms
# A dark theme with a true yellow and red, so the warning and alarm read clearly.
Set Theme { "name": "ringer-demo", "background": "#16161e", "foreground": "#d4d4dc", "cursor": "#d4d4dc", "selection": "#33334a", "black": "#16161e", "red": "#ff4d4d", "green": "#4cd964", "yellow": "#ffd21f", "blue": "#4f8cff", "magenta": "#c77dff", "cyan": "#3fd5e0", "white": "#e6e6ee", "brightBlack": "#6b6b80", "brightRed": "#ff7070", "brightGreen": "#6ee787", "brightYellow": "#ffe066", "brightBlue": "#7aa8ff", "brightMagenta": "#d9a3ff", "brightCyan": "#7ae6ee", "brightWhite": "#ffffff" }

Hide
Type "export PATH=$PWD/target/release:$PATH && clear"
Enter
Show

Sleep 500ms
Type "ringer"
Sleep 400ms
Enter
Sleep 1.5s

# Digits fill in from the right, microwave style.
Type "10"
Sleep 1s
Enter

# Watch it fade from green to red, then ring.
Sleep 13s

Enter
Sleep 1.5s

Hide
Type "q"
