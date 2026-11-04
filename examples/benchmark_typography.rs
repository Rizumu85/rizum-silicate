fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let iterations = args
        .next()
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(30);
    if args.next().is_some() {
        return Err("usage: benchmark_typography [iterations]".into());
    }
    silicate::diagnostics::benchmark_typography(iterations)?;
    Ok(())
}
