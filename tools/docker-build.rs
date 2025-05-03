use std::process::Command;

pub fn main() {
    let tag = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .expect("Failed to fetch git tag");

    if !tag.status.success() {
        eprintln!("git tag checking failed");
        std::process::exit(1);
    }

    let git_tag_str =
        String::from_utf8(tag.stdout).expect("UTF-8 Formatting Error");
    if git_tag_str.trim() == env!("CARGO_PKG_VERSION") {
        println!("Package has same Version, skipping build.");
        return;
    }

    println!("Package has different Version, initiating new Build.");

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
}
