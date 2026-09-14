mod parser;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_usage();
        return ExitCode::SUCCESS;
    }

    if args.len() > 1 {
        eprintln!("diff-rank: too many arguments");
        print_usage();
        return ExitCode::FAILURE;
    }

    let reader: Box<dyn BufRead> = match args.first() {
        Some(path) if path != "-" => match File::open(path) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("diff-rank: {}: {}", path, e);
                return ExitCode::FAILURE;
            }
        },
        _ => Box::new(BufReader::new(io::stdin())),
    };

    let stats = match parser::rank(reader) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("diff-rank: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if stats.is_empty() {
        eprintln!("diff-rank: no file sections found in input");
        return ExitCode::FAILURE;
    }

    for stat in &stats {
        println!(
            "{:>6}  +{:<6} -{:<6}  {}",
            stat.total(),
            stat.added,
            stat.removed,
            stat.path
        );
    }

    ExitCode::SUCCESS
}

fn print_usage() {
    eprintln!("usage: diff-rank [FILE]");
    eprintln!();
    eprintln!("Reads a unified diff from FILE (or stdin if omitted or '-') and");
    eprintln!("prints each changed file ranked by total lines added + removed.");
}
