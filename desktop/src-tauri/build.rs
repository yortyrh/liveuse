use std::path::PathBuf;
use std::process::Command;

fn main() {
    tauri_build::build();
    build_bossac();
}

/// Compile the vendored BOSSA command-line flasher into this crate's output
/// directory. The desktop binary embeds that executable and runs it to write
/// the XIAO, the same tool PlatformIO runs.
fn build_bossac() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("vendor/bossa");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let bindir = out.join("bossa-bin");
    let objdir = out.join("bossa-obj");
    std::fs::create_dir_all(&bindir).unwrap();
    std::fs::create_dir_all(&objdir).unwrap();

    let output = Command::new("make")
        .arg("-C")
        .arg(&src)
        .arg("bossac")
        .arg("VERSION=1.9.1")
        .arg(format!("BINDIR={}", bindir.display()))
        .arg(format!("OBJDIR={}", objdir.display()))
        .arg("CXXFLAGS=-Wno-error")
        .output()
        .unwrap_or_else(|err| panic!("could not run make for bossac: {err}"));
    if !output.status.success() {
        panic!(
            "bossac build failed\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let binary = bindir.join("bossac");
    if !binary.is_file() {
        panic!("bossac build did not produce {}", binary.display());
    }

    println!("cargo:rerun-if-changed=vendor/bossa/Makefile");
    println!("cargo:rerun-if-changed=vendor/bossa/src");
}
