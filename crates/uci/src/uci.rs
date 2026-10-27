//! The UCI loop.
//!
//! Phase 0 answers the handshake (`uci`, `isready`, `quit`) so that a GUI can
//! load the engine. `position` and `go` arrive with the search in Phase 2.
//! Unknown commands are ignored, as the protocol requires.

use std::io::{self, BufRead, Write};

/// Read commands until `quit` or end of input, writing replies to `out`.
pub fn run<R: BufRead, W: Write>(input: R, mut out: W) -> io::Result<()> {
    for line in input.lines() {
        let line = line?;
        let mut words = line.split_whitespace();
        match words.next() {
            Some("uci") => {
                writeln!(out, "id name rusty-chess {}", env!("CARGO_PKG_VERSION"))?;
                writeln!(out, "id author {}", env!("CARGO_PKG_AUTHORS"))?;
                writeln!(out, "uciok")?;
            }
            Some("isready") => writeln!(out, "readyok")?,
            Some("quit") => break,
            _ => {}
        }
        out.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(input: &str) -> String {
        let mut out = Vec::new();
        run(input.as_bytes(), &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn handshake() {
        let out = drive("uci\n");
        assert!(out.starts_with("id name rusty-chess "));
        assert!(out.ends_with("uciok\n"));
    }

    #[test]
    fn quit_stops_reading() {
        assert_eq!(drive("isready\nquit\nisready\n"), "readyok\n");
    }

    #[test]
    fn leading_whitespace_and_unknown_commands() {
        assert_eq!(
            drive("  isready  \nsetoption name Hash value 1\n"),
            "readyok\n"
        );
    }
}
