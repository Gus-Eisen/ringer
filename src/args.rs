//! Command-line arguments: an optional duration to start counting down from.

use std::time::Duration;

/// The longest time the clock can show: 99:59:59.
pub const MAX: Duration = Duration::from_secs(99 * 3600 + 59 * 60 + 59);

pub const USAGE: &str = "\
ringer — a terminal timer that warns you when you won't hear the alarm

Usage:
  ringer              set the time on screen, then press Enter
  ringer <duration>   start counting down right away

Durations:
  25m  1h30m  1h 30m  90s  90   (a bare number is seconds)
  1:30  1:02:03                 (minutes:seconds or hours:minutes:seconds)

Options:
  -h, --help          show this help
  -V, --version       show the version";

pub enum Command {
    /// Open the timer, starting right away if a duration was given.
    Run(Option<Duration>),
    Help,
    Version,
}

/// Parses everything after the program name. Several arguments are joined,
/// so `ringer 1h 30m` works the same as `ringer 1h30m`.
pub fn parse(args: &[String]) -> Result<Command, String> {
    match args {
        [] => Ok(Command::Run(None)),
        [flag] if flag == "-h" || flag == "--help" => Ok(Command::Help),
        [flag] if flag == "-V" || flag == "--version" => Ok(Command::Version),
        _ => {
            if let Some(flag) = args.iter().find(|a| a.starts_with('-')) {
                return Err(format!("unknown option '{flag}'"));
            }
            parse_duration(&args.concat()).map(|d| Command::Run(Some(d)))
        }
    }
}

pub fn parse_duration(input: &str) -> Result<Duration, String> {
    let s: String = input.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    let invalid = || format!("'{input}' isn't a duration — try 25m, 1h30m, 90s or 1:30");

    let secs = if s.contains(':') {
        // m:s or h:m:s
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() > 3 || parts.iter().any(|p| p.is_empty()) {
            return Err(invalid());
        }
        let mut total = 0u64;
        for part in parts {
            let n: u64 = part.parse().map_err(|_| invalid())?;
            total = total.checked_mul(60).and_then(|t| t.checked_add(n)).ok_or_else(invalid)?;
        }
        total
    } else if s.bytes().all(|b| b.is_ascii_digit()) && !s.is_empty() {
        s.parse().map_err(|_| invalid())?
    } else {
        // A sequence of <number><unit>, each unit at most once, largest first.
        let mut total = 0u64;
        let mut number = String::new();
        let mut last_unit = u64::MAX;
        for c in s.chars() {
            if c.is_ascii_digit() {
                number.push(c);
                continue;
            }
            let unit = match c {
                'h' => 3600,
                'm' => 60,
                's' => 1,
                _ => return Err(invalid()),
            };
            if number.is_empty() || unit >= last_unit {
                return Err(invalid());
            }
            let n: u64 = number.parse().map_err(|_| invalid())?;
            total = n.checked_mul(unit).and_then(|v| v.checked_add(total)).ok_or_else(invalid)?;
            number.clear();
            last_unit = unit;
        }
        if !number.is_empty() {
            return Err(invalid());
        }
        total
    };

    let duration = Duration::from_secs(secs);
    if duration.is_zero() {
        Err("the duration has to be longer than 0 seconds".into())
    } else if duration > MAX {
        Err("the longest timer is 99:59:59".into())
    } else {
        Ok(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: &str) -> u64 {
        parse_duration(s).unwrap().as_secs()
    }

    #[test]
    fn units() {
        assert_eq!(secs("25m"), 1500);
        assert_eq!(secs("1h30m"), 5400);
        assert_eq!(secs("1h30m15s"), 5415);
        assert_eq!(secs("90s"), 90);
        assert_eq!(secs("2H"), 7200);
        assert_eq!(secs("90m"), 5400);
        assert_eq!(secs("1h 30m"), 5400);
    }

    #[test]
    fn bare_number_is_seconds() {
        assert_eq!(secs("90"), 90);
    }

    #[test]
    fn colons() {
        assert_eq!(secs("1:30"), 90);
        assert_eq!(secs("1:02:03"), 3723);
        assert_eq!(secs("99:59:59"), MAX.as_secs());
    }

    #[test]
    fn rejects_bad_input() {
        for bad in ["", "abc", "m", "5x", "1m1h", "1m1m", "1h30", "1::2", "1:2:3:4", ":30", "0", "0m", "100h", "-5m"] {
            assert!(parse_duration(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn args_are_joined() {
        let args = ["1h".to_string(), "30m".to_string()];
        assert!(matches!(parse(&args), Ok(Command::Run(Some(d))) if d.as_secs() == 5400));
    }

    #[test]
    fn flags() {
        assert!(matches!(parse(&["--help".into()]), Ok(Command::Help)));
        assert!(matches!(parse(&["-V".into()]), Ok(Command::Version)));
        assert!(parse(&["--nope".into()]).is_err());
        assert!(matches!(parse(&[]), Ok(Command::Run(None))));
    }
}
