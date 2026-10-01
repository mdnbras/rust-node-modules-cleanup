use rust_node_modules_cleanup::{
    cli::{self, Command},
    format_size, scan,
};
use std::{
    io::{self, Write},
    process::ExitCode,
    thread,
    time::Instant,
};

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let options = match cli::parse(std::env::args_os().skip(1))? {
        Command::Help => {
            println!("{}", cli::HELP);
            return Ok(true);
        }
        Command::Version => {
            println!("rust-node-modules-cleanup {}", env!("CARGO_PKG_VERSION"));
            return Ok(true);
        }
        Command::Run(options) => options,
    };
    eprintln!("Scanning {}...", options.path.display());
    let result = scan::scan(&options.path, options.jobs)?;
    let warnings: Vec<_> = result
        .warnings
        .iter()
        .chain(result.candidates.iter().flat_map(|c| c.warnings.iter()))
        .collect();
    for warning in &warnings {
        eprintln!("Warning: {warning}");
    }
    let complete = warnings.is_empty();
    if result.candidates.is_empty() {
        println!("No node_modules directories found.");
        return Ok(complete);
    }
    println!("{:>14}  PATH", "SIZE");
    for candidate in &result.candidates {
        // Debug formatting escapes control characters in untrusted filenames.
        println!("{:>14}  {:?}", format_size(candidate.bytes), candidate.path);
    }
    println!(
        "{} directories; estimated file size: {}{}",
        result.candidates.len(),
        format_size(result.total_bytes()),
        if complete { "" } else { " (incomplete scan)" }
    );
    if options.dry {
        println!("Dry run: nothing was deleted.");
        return Ok(complete);
    }
    if !options.yes {
        print!("Permanently delete ALL listed directories? [y/N] ");
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !cli::confirmed(&answer) {
            println!("Cancelled: nothing was deleted.");
            return Ok(complete);
        }
    }
    let start = Instant::now();
    let (mut deleted, mut bytes, mut failed) = (0, 0u64, 0);
    for batch in result.candidates.chunks(options.jobs) {
        let outcomes = thread::scope(|scope| {
            let handles: Vec<_> = batch
                .iter()
                .map(|candidate| {
                    let result = &result;
                    scope.spawn(move || (candidate, result.delete(candidate)))
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("cleanup worker panicked"))
                .collect::<Vec<_>>()
        });
        for (candidate, outcome) in outcomes {
            match outcome {
                Ok(()) => {
                    deleted += 1;
                    bytes = bytes.saturating_add(candidate.bytes);
                }
                Err(e) => {
                    failed += 1;
                    eprintln!("Failed to delete {:?}: {e}", candidate.path);
                }
            }
        }
        eprintln!(
            "Processed {}/{} directories",
            deleted + failed,
            result.candidates.len()
        );
    }
    println!(
        "Deleted: {deleted}; failed: {failed}; estimated removed file size: {}; time: {:.2}s",
        format_size(bytes),
        start.elapsed().as_secs_f64()
    );
    Ok(complete && failed == 0)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}
