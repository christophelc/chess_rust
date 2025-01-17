use std::process::Command;
use std::time::SystemTime;

fn main() {
    // Get current UTC timestamp
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    // Convert timestamp to string
    let timestamp = now.as_secs().to_string();

    // Get the last Git commit hash
    let git_commit_hash = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    // Set the build date as an environment variable for compile time
    println!("cargo:rustc-env=BUILD_DATE={}", timestamp);
    // Set the Git commit hash as an environment variable for compile time
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash);
}
