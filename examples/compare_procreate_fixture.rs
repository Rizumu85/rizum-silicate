use silicate::diagnostics::compare_procreate_fixture;
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1).map(PathBuf::from);
    let fixture = args
        .next()
        .ok_or("usage: compare_procreate_fixture <document.procreate> [output-directory]")?;
    let output_directory = args.next();
    if args.next().is_some() {
        return Err("too many arguments".into());
    }

    let report = compare_procreate_fixture(&fixture, output_directory.as_deref())?;
    println!("verification=procreate_render_comparison_v1");
    println!("adapter={}", report.adapter);
    println!("fixture={}", fixture.display());
    println!(
        "rendered_dimensions={}x{}",
        report.rendered_dimensions.0, report.rendered_dimensions.1
    );
    println!(
        "quicklook_dimensions={}x{}",
        report.quicklook_dimensions.0, report.quicklook_dimensions.1
    );
    println!(
        "quicklook_compared_pixels={}",
        report.quicklook_compared_pixels
    );
    println!(
        "quicklook_pixels_over_four_lsb={}",
        report.quicklook_pixels_over_four_lsb
    );
    println!(
        "quicklook_mean_absolute_error={:.4}",
        report.quicklook_mean_absolute_error
    );
    println!(
        "quicklook_root_mean_square_error={:.4}",
        report.quicklook_root_mean_square_error
    );
    println!(
        "quicklook_max_channel_error={}",
        report.quicklook_max_channel_error
    );
    println!(
        "composite_compared_pixels={}",
        report.composite_compared_pixels
    );
    println!(
        "composite_pixels_over_four_lsb={}",
        report.composite_pixels_over_four_lsb
    );
    println!(
        "composite_mean_absolute_error={:.4}",
        report.composite_mean_absolute_error
    );
    println!(
        "composite_root_mean_square_error={:.4}",
        report.composite_root_mean_square_error
    );
    println!(
        "composite_max_channel_error={}",
        report.composite_max_channel_error
    );
    if let Some(output_directory) = output_directory {
        println!("output_directory={}", output_directory.display());
    }
    Ok(())
}
