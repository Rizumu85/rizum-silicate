use std::{io, process::Command};

fn main() -> io::Result<()> {
    built::write_built_file().expect("Failed to acquire build-time information");
    if let Ok(output) = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        && output.status.success()
        && let Ok(hash) = String::from_utf8(output.stdout)
    {
        println!("cargo:rustc-env=SILICATE_GIT_HASH={}", hash.trim());
    }

    #[cfg(windows)]
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        winres::WindowsResource::new()
            .set_icon("assets/favicon.ico")
            .set("ProductName", "Rizum Silicate")
            .set("FileDescription", "Rizum Silicate")
            .set("CompanyName", "Rizum")
            .set("InternalName", "Rizum Silicate")
            .set("OriginalFilename", "silicate.exe")
            .set(
                "LegalCopyright",
                "Copyright (c) Rizum and Silicate contributors",
            )
            .compile()?;
    }
    Ok(())
}
