mod parser;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::ExitCode;

struct Options {
    path: Option<String>,
    top: Option<usize>,
    min: u64,
}

enum Parsed {
    Run(Options),
    Help,
}

/// Parses the value of a numeric flag, taken either from `--flag=N` or from
/// the next argument.
fn flag_value<T: std::str::FromStr>(
    flag: &str,
    inline: Option<&str>,
    rest: &mut impl Iterator<Item = String>,
) -> Result<T, String> {
    let raw = match inline {
        Some(v) => v.to_string(),
        None => rest
            .next()
            .ok_or_else(|| format!("{} needs a value", flag))?,
    };
    raw.parse()
        .map_err(|_| format!("{}: invalid number '{}'", flag, raw))
}

fn parse_args(args: Vec<String>) -> Result<Parsed, String> {
    let mut opts = Options {
        path: None,
        top: None,
        min: 0,
    };
    let mut positional = Vec::new();
    let mut it = args.into_iter();

    while let Some(arg) = it.next() {
        if arg == "-h" || arg == "--help" {
            return Ok(Parsed::Help);
        }
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n, Some(v)),
            _ => (arg.as_str(), None),
        };
        match name {
            "--top" => opts.top = Some(flag_value("--top", inline, &mut it)?),
            "--min" => opts.min = flag_value("--min", inline, &mut it)?,
            _ => positional.push(arg),
        }
    }

    if positional.len() > 1 {
        return Err("too many arguments".to_string());
    }
    opts.path = positional.pop();
    Ok(Parsed::Run(opts))
}

fn main() -> ExitCode {
    let opts = match parse_args(env::args().skip(1).collect()) {
        Ok(Parsed::Run(o)) => o,
        Ok(Parsed::Help) => {
            print_usage();
            return ExitCode::SUCCESS;
        }
        Err(msg) => {
            eprintln!("diff-rank: {}", msg);
            print_usage();
            return ExitCode::FAILURE;
        }
    };

    let reader: Box<dyn BufRead> = match &opts.path {
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

    // Filter after the "no file sections" check so an over-aggressive --min
    // prints nothing without being reported as a parse failure.
    let shown = stats
        .iter()
        .filter(|s| s.total() >= opts.min)
        .take(opts.top.unwrap_or(usize::MAX));

    for stat in shown {
        if stat.is_binary {
            println!(
                "{:>6}  {:<17}{}",
                stat.total(),
                "binary",
                stat.display_path()
            );
        } else {
            println!(
                "{:>6}  +{:<6} -{:<6}  {}",
                stat.total(),
                stat.added,
                stat.removed,
                stat.display_path()
            );
        }
    }

    ExitCode::SUCCESS
}

fn print_usage() {
    eprintln!("usage: diff-rank [--top N] [--min N] [FILE]");
    eprintln!();
    eprintln!("Reads a unified diff from FILE (or stdin if omitted or '-') and");
    eprintln!("prints each changed file ranked by total lines added + removed.");
    eprintln!();
    eprintln!("  --top N   show only the N most changed files");
    eprintln!("  --min N   skip files with fewer than N lines changed");
}
