use silicate::diagnostics::verify_rendering_fixtures;
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let mask_fixture = args
        .next()
        .ok_or("usage: verify_fixture_rendering <mask.procreate> <clipping.procreate>")?;
    let clipping_fixture = args
        .next()
        .ok_or("usage: verify_fixture_rendering <mask.procreate> <clipping.procreate>")?;
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let report = verify_rendering_fixtures(&mask_fixture, &clipping_fixture)?;
    println!("verification=fixture_rendering_v2");
    println!("adapter={}", report.adapter);
    println!("mask_fixture={}", mask_fixture.display());
    println!("mask_changed_pixels={}", report.mask_changed_pixels);
    println!("clipping_fixture={}", clipping_fixture.display());
    println!("group_changed_pixels={}", report.group_changed_pixels);
    println!("clipping_changed_pixels={}", report.clipping_changed_pixels);
    println!(
        "clipping_base_visibility_changed_pixels={}",
        report.clipping_base_visibility_changed_pixels
    );
    println!("clipping_topology_cases={}", report.clipping_topology_cases);
    println!("transparent_pixels={}", report.transparent_pixels);
    println!("partial_alpha_pixels={}", report.partial_alpha_pixels);
    println!("opaque_pixels={}", report.opaque_pixels);
    Ok(())
}
