use std::process::Command;


pub fn main() {
    let status = Command::new("docker")
        .args(["build", "-f", "docker/Dockerfile.bot", "-t", "codeberg.org/mto/fia-docs-bot:latest", "."])
        .status()
        .expect("Failed to run Docker build");

    if !status.success() {
        eprintln!("Docker build failed!");
        std::process::exit(1);
    }
}
