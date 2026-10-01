//! Noninteractive adapter for benchmarks/run.py; deletes the supplied fixture.
use rust_node_modules_cleanup::scan;
use std::{env, io, path::PathBuf, thread, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env::args_os().nth(1).ok_or("missing fixture path")?);
    let started = Instant::now();
    let result = scan::scan(&path, 5)?;
    if !result.warnings.is_empty() || result.candidates.iter().any(|c| !c.warnings.is_empty()) {
        return Err(io::Error::other("incomplete benchmark scan").into());
    }
    let scanned = Instant::now();
    // Match the production CLI's default of five deletions per batch.
    for batch in result.candidates.chunks(5) {
        thread::scope(|scope| -> io::Result<()> {
            let handles: Vec<_> = batch
                .iter()
                .map(|candidate| {
                    let result = &result;
                    scope.spawn(move || result.delete(candidate))
                })
                .collect();
            for handle in handles {
                handle.join().expect("benchmark worker panicked")?;
            }
            Ok(())
        })?;
    }
    let deleted = Instant::now();
    println!(
        "{{\"folders\":{},\"bytes\":{},\"scan_ms\":{},\"delete_ms\":{},\"core_ms\":{}}}",
        result.candidates.len(),
        result.total_bytes(),
        scanned.duration_since(started).as_secs_f64() * 1000.0,
        deleted.duration_since(scanned).as_secs_f64() * 1000.0,
        deleted.duration_since(started).as_secs_f64() * 1000.0,
    );
    Ok(())
}
