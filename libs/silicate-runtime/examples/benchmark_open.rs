use silicate_runtime::DocumentRuntime;
use std::env;
use std::hint::black_box;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "usage: benchmark_open <document.procreate> [iterations]".to_owned())?;
    let iterations = args
        .next()
        .map(|value| {
            value
                .to_string_lossy()
                .parse::<usize>()
                .map_err(|_| "iterations must be a positive integer".to_owned())
        })
        .transpose()?
        .unwrap_or(5);
    if iterations == 0 {
        return Err("iterations must be a positive integer".to_owned());
    }
    if args.next().is_some() {
        return Err("usage: benchmark_open <document.procreate> [iterations]".to_owned());
    }

    let bytes = std::fs::read(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;

    let warmup = DocumentRuntime::new()
        .open(black_box(&bytes))
        .map_err(|error| format!("warmup failed: {error}"))?;
    let title = warmup
        .value
        .title
        .as_deref()
        .unwrap_or("Untitled")
        .to_owned();
    let layer_count = warmup.value.layer_count;
    let snapshot_node_count = warmup.value.layers.len();
    drop(warmup);

    let mut samples = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let mut runtime = DocumentRuntime::new();
        let started = Instant::now();
        let opened = runtime
            .open(black_box(&bytes))
            .map_err(|error| format!("timed open failed: {error}"))?;
        samples.push(started.elapsed());
        black_box(opened);
    }

    samples.sort_unstable();
    let total: Duration = samples.iter().sum();
    let mean = total / iterations as u32;
    let median = median(&samples);

    println!("benchmark=silicate_runtime_open_v1");
    println!("fixture={}", path.display());
    println!("fixture_bytes={}", bytes.len());
    println!("title={title}");
    println!("layer_count={layer_count}");
    println!("snapshot_node_count={snapshot_node_count}");
    println!("iterations={iterations}");
    println!("min_ms={:.3}", duration_ms(samples[0]));
    println!("median_ms={:.3}", duration_ms(median));
    println!("mean_ms={:.3}", duration_ms(mean));
    println!(
        "max_ms={:.3}",
        duration_ms(*samples.last().expect("iterations is non-zero"))
    );

    Ok(())
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn median(sorted_samples: &[Duration]) -> Duration {
    let midpoint = sorted_samples.len() / 2;
    if sorted_samples.len().is_multiple_of(2) {
        (sorted_samples[midpoint - 1] + sorted_samples[midpoint]) / 2
    } else {
        sorted_samples[midpoint]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_averages_the_middle_pair_for_even_samples() {
        let samples = [
            Duration::from_millis(1),
            Duration::from_millis(2),
            Duration::from_millis(4),
            Duration::from_millis(8),
        ];

        assert_eq!(median(&samples), Duration::from_millis(3));
    }

    #[test]
    fn median_uses_the_middle_value_for_odd_samples() {
        let samples = [
            Duration::from_millis(1),
            Duration::from_millis(3),
            Duration::from_millis(9),
        ];

        assert_eq!(median(&samples), Duration::from_millis(3));
    }
}
