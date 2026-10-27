//! Command-line parsing. Hand-rolled: two subcommands do not justify a
//! dependency in the engine binary.

use std::fmt;
use std::path::PathBuf;

/// The usage text printed by `--help` and after a usage error.
pub const USAGE: &str = "\
Usage: rusty-chess [COMMAND] [OPTIONS]

With no command, speaks UCI on stdin/stdout.

Commands:
  bench      Search the fixed position set and print the node signature
               --depth N   depth per position (default 12)
               --hash MB   hash size in MB (default 16)
               --out FILE  write JSON lines telemetry to FILE
  perft      Count leaf nodes from a position
               --depth N   depth (default 5)
               --fen FEN   position (default: the starting position)

Options:
  --version  Print the version
  --help     Print this help";

/// A parsed command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Run the UCI loop.
    Uci,
    /// Run the deterministic bench.
    Bench(BenchArgs),
    /// Run perft.
    Perft(PerftArgs),
    /// Print the version.
    Version,
    /// Print the usage text.
    Help,
}

/// Arguments to `bench`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchArgs {
    /// Depth per position.
    pub depth: u32,
    /// Hash size in MB.
    pub hash_mb: u32,
    /// Where to write JSON lines telemetry, if anywhere.
    pub out: Option<PathBuf>,
}

impl Default for BenchArgs {
    fn default() -> Self {
        Self {
            depth: 12,
            hash_mb: 16,
            out: None,
        }
    }
}

/// Arguments to `perft`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PerftArgs {
    /// Depth.
    pub depth: u32,
    /// The position, or `None` for the starting position.
    pub fen: Option<String>,
}

impl Default for PerftArgs {
    fn default() -> Self {
        Self {
            depth: 5,
            fen: None,
        }
    }
}

/// A usage error: what was wrong with the arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageError(String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UsageError {}

/// Parse the arguments after the program name.
pub fn parse<I>(args: I) -> Result<Command, UsageError>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Command::Uci);
    };
    match first.as_str() {
        "--version" | "-V" => Ok(Command::Version),
        "--help" | "-h" => Ok(Command::Help),
        "bench" => parse_bench(args).map(Command::Bench),
        "perft" => parse_perft(args).map(Command::Perft),
        other => Err(UsageError(format!("unknown command `{other}`"))),
    }
}

fn parse_bench<I: Iterator<Item = String>>(mut args: I) -> Result<BenchArgs, UsageError> {
    let mut out = BenchArgs::default();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--depth" => out.depth = value(&flag, args.next())?,
            "--hash" => out.hash_mb = value(&flag, args.next())?,
            "--out" => out.out = Some(PathBuf::from(raw(&flag, args.next())?)),
            other => return Err(UsageError(format!("unknown option `{other}` for bench"))),
        }
    }
    if out.depth == 0 {
        return Err(UsageError("--depth must be at least 1".into()));
    }
    if out.hash_mb == 0 {
        return Err(UsageError("--hash must be at least 1".into()));
    }
    Ok(out)
}

fn parse_perft<I: Iterator<Item = String>>(mut args: I) -> Result<PerftArgs, UsageError> {
    let mut out = PerftArgs::default();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--depth" => out.depth = value(&flag, args.next())?,
            "--fen" => out.fen = Some(raw(&flag, args.next())?),
            other => return Err(UsageError(format!("unknown option `{other}` for perft"))),
        }
    }
    Ok(out)
}

fn raw(flag: &str, next: Option<String>) -> Result<String, UsageError> {
    next.ok_or_else(|| UsageError(format!("`{flag}` needs a value")))
}

fn value<T: std::str::FromStr>(flag: &str, next: Option<String>) -> Result<T, UsageError> {
    let text = raw(flag, next)?;
    text.parse()
        .map_err(|_| UsageError(format!("`{flag}` got `{text}`, which is not a valid value")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Command, UsageError> {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_arguments_means_uci() {
        assert_eq!(p(&[]), Ok(Command::Uci));
    }

    #[test]
    fn bench_defaults_and_overrides() {
        assert_eq!(p(&["bench"]), Ok(Command::Bench(BenchArgs::default())));
        assert_eq!(
            p(&["bench", "--depth", "3", "--hash", "64", "--out", "b.jsonl"]),
            Ok(Command::Bench(BenchArgs {
                depth: 3,
                hash_mb: 64,
                out: Some(PathBuf::from("b.jsonl")),
            }))
        );
    }

    #[test]
    fn perft_defaults_and_overrides() {
        assert_eq!(p(&["perft"]), Ok(Command::Perft(PerftArgs::default())));
        let fen = "8/8/8/8/8/8/8/K6k w - - 0 1";
        assert_eq!(
            p(&["perft", "--fen", fen, "--depth", "2"]),
            Ok(Command::Perft(PerftArgs {
                depth: 2,
                fen: Some(fen.into()),
            }))
        );
    }

    #[test]
    fn usage_errors_name_the_problem() {
        assert!(
            p(&["bench", "--depth"])
                .unwrap_err()
                .to_string()
                .contains("--depth")
        );
        assert!(
            p(&["bench", "--depth", "x"])
                .unwrap_err()
                .to_string()
                .contains("`x`")
        );
        assert!(p(&["bench", "--depth", "0"]).is_err());
        assert!(p(&["bench", "--bogus"]).is_err());
        assert!(p(&["perft", "--hash", "1"]).is_err());
        assert!(
            p(&["frobnicate"])
                .unwrap_err()
                .to_string()
                .contains("frobnicate")
        );
    }

    #[test]
    fn version_and_help() {
        assert_eq!(p(&["--version"]), Ok(Command::Version));
        assert_eq!(p(&["-V"]), Ok(Command::Version));
        assert_eq!(p(&["--help"]), Ok(Command::Help));
    }
}
