//! The `rusty-chess` binary (the workspace root package): a UCI engine with
//! `bench` and `perft` subcommands.
//!
//! ```text
//! rusty-chess                     # UCI loop on stdin/stdout
//! rusty-chess bench [--depth N] [--hash MB] [--out FILE]
//! rusty-chess perft [--depth N] [--fen FEN]
//! rusty-chess --version | --help
//! ```

mod bench;
mod cli;
mod perft;
mod positions;
mod uci;

use std::io;
use std::process::ExitCode;

use cli::Command;

/// Exit code for a usage error (bad arguments).
const EXIT_USAGE: u8 = 2;
/// Exit code when the perft suite does not match.
const EXIT_PERFT_MISMATCH: u8 = 3;

fn main() -> ExitCode {
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("rusty-chess: {err}");
            eprintln!("{}", cli::USAGE);
            return ExitCode::from(EXIT_USAGE);
        }
    };

    let result = match command {
        Command::Version => {
            println!("rusty-chess {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Help => {
            println!("{}", cli::USAGE);
            Ok(())
        }
        Command::Uci => uci::run(io::stdin().lock(), io::stdout().lock()),
        Command::Bench(args) => bench::run(&args, &mut io::stdout().lock()).map(drop),
        Command::Perft(args) => match perft::run(&args, &mut io::stdout().lock()) {
            Ok(true) => Ok(()),
            Ok(false) => return ExitCode::from(EXIT_PERFT_MISMATCH),
            Err(e) => Err(e),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("rusty-chess: {err}");
            ExitCode::FAILURE
        }
    }
}
