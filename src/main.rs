mod alarm;
mod font;
mod sound;

use std::time::{Duration, Instant};

use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, BorderType, Paragraph},
};

use alarm::Alarm;
use sound::MuteMonitor;

/// Maximum number of digits the user can type (HHMMSS).
const MAX_DIGITS: usize = 6;
const WAVE: &str = "*~";

enum Mode {
    /// Entering a time. Digits fill from the right, microwave style.
    Input,
    Running { end: Instant, total: Duration },
    /// Holding the Alarm keeps it ringing; dropping it stops the sound.
    Ringing { _alarm: Alarm },
}

struct App {
    digits: String,
    mode: Mode,
    mute: MuteMonitor,
    started: Instant,
    quit: bool,
}

impl App {
    fn new() -> Self {
        Self {
            digits: String::new(),
            mode: Mode::Input,
            mute: MuteMonitor::start(),
            started: Instant::now(),
            quit: false,
        }
    }

    /// The input digits padded to six places as (hours, minutes, seconds).
    fn input_hms(&self) -> (u64, u64, u64) {
        let padded = format!("{:0>6}", self.digits);
        let part = |r: std::ops::Range<usize>| padded[r].parse::<u64>().unwrap_or(0);
        (part(0..2), part(2..4), part(4..6))
    }

    fn input_duration(&self) -> Duration {
        let (h, m, s) = self.input_hms();
        Duration::from_secs(h * 3600 + m * 60 + s)
    }

    fn tick(&mut self) {
        if let Mode::Running { end, .. } = self.mode {
            if Instant::now() >= end {
                self.mode = Mode::Ringing { _alarm: Alarm::start() };
            }
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }
        match &self.mode {
            Mode::Input => match code {
                KeyCode::Char(c @ '0'..='9') => {
                    if self.digits.len() < MAX_DIGITS && !(self.digits.is_empty() && c == '0') {
                        self.digits.push(c);
                    }
                }
                KeyCode::Backspace => {
                    self.digits.pop();
                }
                KeyCode::Delete => self.digits.clear(),
                KeyCode::Enter => {
                    let duration = self.input_duration();
                    if !duration.is_zero() {
                        self.mute.check_now();
                        self.mode = Mode::Running { end: Instant::now() + duration, total: duration };
                    }
                }
                KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
                _ => {}
            },
            Mode::Running { .. } => match code {
                KeyCode::Esc => self.mode = Mode::Input,
                KeyCode::Char('q') => self.quit = true,
                _ => {}
            },
            Mode::Ringing { .. } => match code {
                // Dropping the Alarm stops the sound.
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Esc => self.mode = Mode::Input,
                KeyCode::Char('q') => self.quit = true,
                _ => {}
            },
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
        while !self.quit {
            self.tick();
            terminal.draw(|f| self.draw(f))?;
            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key.code, key.modifiers);
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .title(Line::from(" rust-timer ").bold().centered());
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let [warning, _, clock, _, anim, _, help] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(font::HEIGHT as u16),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        if let Some(msg) = self.mute.warning() {
            frame.render_widget(
                Paragraph::new(format!("⚠  {msg}"))
                    .centered()
                    .style(Style::new().fg(Color::Black).bg(Color::Yellow).bold()),
                warning,
            );
        }

        let elapsed = self.started.elapsed().as_millis();
        let (time_text, style, anim_line, help_text) = match &self.mode {
            Mode::Input => {
                let (h, m, s) = self.input_hms();
                (
                    format!("{h:02}:{m:02}:{s:02}"),
                    Style::new().fg(Color::Cyan),
                    Line::from("type digits to set hours:minutes:seconds").dim(),
                    "0-9 set time · Backspace erase · Del clear · Enter start · q quit",
                )
            }
            Mode::Running { end, total } => {
                let left = end.saturating_duration_since(Instant::now());
                let remaining = left.as_secs_f64() / total.as_secs_f64();
                // Round up so the display shows 00:00:01 until the very end.
                let secs = left.as_secs() + u64::from(left.subsec_nanos() > 0);
                let offset = (elapsed / 150) as usize;
                (
                    format!("{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60),
                    Style::new().fg(countdown_color(remaining)),
                    wave(offset, inner.width.saturating_sub(4) as usize),
                    "Esc cancel · q quit",
                )
            }
            Mode::Ringing { .. } => {
                let flash = (elapsed / 400) % 2 == 0;
                let color = if flash { Color::Red } else { Color::White };
                (
                    "00:00:00".to_string(),
                    Style::new().fg(color),
                    Line::from("⏰  TIME'S UP!  ⏰").fg(color).bold(),
                    "Enter / Space / Esc stop alarm · q quit",
                )
            }
        };

        render_clock(frame, clock, &time_text, style);
        frame.render_widget(Paragraph::new(anim_line).centered(), anim);
        frame.render_widget(Paragraph::new(help_text).centered().dim(), help);
    }
}

/// Fades from green (all time left) through yellow to red (none left).
/// `remaining` is the fraction of the total duration still to go.
fn countdown_color(remaining: f64) -> Color {
    // Hue 120° (green) down to 0° (red) at full saturation and brightness.
    let hue = 120.0 * remaining.clamp(0.0, 1.0);
    let (r, g) = if hue >= 60.0 { ((120.0 - hue) / 60.0, 1.0) } else { (1.0, hue / 60.0) };
    if supports_truecolor() {
        Color::Rgb((r * 255.0).round() as u8, (g * 255.0).round() as u8, 0)
    } else {
        // Nearest colour in the 6x6x6 cube of the 256-colour palette.
        let level = |v: f64| (v * 5.0).round() as u8;
        Color::Indexed(16 + 36 * level(r) + 6 * level(g))
    }
}

fn supports_truecolor() -> bool {
    static TRUECOLOR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRUECOLOR.get_or_init(|| {
        std::env::var("COLORTERM").is_ok_and(|v| v == "truecolor" || v == "24bit")
    })
}

/// A `*~*~*~` wave scrolling to the right, coloured with a moving gradient.
fn wave(offset: usize, width: usize) -> Line<'static> {
    let colors = [Color::Cyan, Color::LightCyan, Color::LightBlue, Color::Blue];
    let chars: Vec<char> = WAVE.chars().collect();
    let spans = (0..width).map(|i| {
        let pos = i + chars.len() * 1000 - offset % (chars.len() * 1000);
        let c = chars[pos % chars.len()];
        let color = colors[(i / 3 + colors.len() * 1000 - offset % (colors.len() * 1000)) % colors.len()];
        ratatui::text::Span::styled(c.to_string(), Style::new().fg(color).add_modifier(Modifier::BOLD))
    });
    Line::from_iter(spans)
}

/// Draws the time in large block digits, falling back to plain text if the
/// terminal is too narrow.
fn render_clock(frame: &mut Frame, area: Rect, text: &str, style: Style) {
    let big = font::render(text);
    let width = big.first().map_or(0, |l| l.chars().count()) as u16;
    if width <= area.width {
        let [area] = Layout::horizontal([Constraint::Length(width)]).flex(Flex::Center).areas(area);
        let lines: Vec<Line> = big.into_iter().map(Line::from).collect();
        frame.render_widget(Paragraph::new(Text::from(lines)).style(style), area);
    } else {
        let [_, mid, _] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1), Constraint::Fill(1)]).areas(area);
        frame.render_widget(Paragraph::new(text.to_string()).centered().style(style.bold()), mid);
    }
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal);
    ratatui::restore();
    result
}
