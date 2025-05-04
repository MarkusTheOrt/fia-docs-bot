use std::process::Command;

pub fn main() {
    let mut arg = std::env::args();

    if arg.len() < 2 {
        eprintln!("This program requires the repository token to be passed.");
        std::process::exit(1);
    }

    let repo = arg.next_back().unwrap().to_lowercase();

    let tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .expect("Failed to fetch git tag");

    if !tag.status.success() {
        eprintln!("git tag checking failed");
        std::process::exit(1);
    }

    let status = Command::new("docker")
        .env("DOCKER_BAKE", "1")
        .args([
            "build",
            "-f",
            "docker/Dockerfile.bot",
            "-t",
            env!("CARGO_PKG_NAME"),
            ".",
        ])
        .status()
        .expect("Failed to run Docker build");

    if !status.success() {
        eprintln!("Docker build failed!");
        std::process::exit(1);
    }

    let status = Command::new("docker")
        .args([
            "tag",
            env!("CARGO_PKG_NAME"),
            &format!(
                "ghcr.io/{}/{}:{}",
                repo,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ),
        ])
        .status()
        .expect("Failed to run Docker tag");

    if !status.success() {
        eprintln!("Docker rename failed!");
        std::process::exit(1);
    }
    let status = Command::new("docker")
        .args([
            "tag",
            env!("CARGO_PKG_NAME"),
            &format!("ghcr.io/{}/{}:latest", repo, env!("CARGO_PKG_NAME"),),
        ])
        .status()
        .expect("Failed to run Docker tag");

    if !status.success() {
        eprintln!("Docker rename failed!");
        std::process::exit(1);
    }

    let status = Command::new("docker")
        .args([
            "push",
            &format!(
                "ghcr.io/{}/{}:{}",
                repo,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ),
        ])
        .status()
        .expect("Failed to run Docker tag");

    if !status.success() {
        eprintln!("Docker push failed!");
        std::process::exit(1);
    }
    let status = Command::new("docker")
        .args([
            "push",
            &format!("ghcr.io/{}/{}:latest", repo, env!("CARGO_PKG_NAME"),),
        ])
        .status()
        .expect("Failed to run Docker tag");

    if !status.success() {
        eprintln!("Docker push failed!");
        std::process::exit(1);
    }
}
