use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let fixture = PathBuf::from(
        args.next()
            .ok_or("usage: benchmark_document_open <file.procreate> [iterations]")?,
    );
    let iterations = args
        .next()
        .map(|value| value.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(10);
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    silicate::diagnostics::benchmark_document_open(&fixture, iterations)?;
    Ok(())
}
