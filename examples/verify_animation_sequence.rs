use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let fixture = args
        .next()
        .ok_or("usage: verify_animation_sequence <animation.procreate> <new-output-folder>")?;
    let output = args.next().ok_or("missing output folder")?;
    std::fs::create_dir(&output)?;
    silicate::diagnostics::verify_animation_sequence(&fixture, &output)?;
    println!("verification=animation_sequence_v1");
    Ok(())
}
