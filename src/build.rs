// Put data in the binary at compile time.
// For example, the building script can input the last git commit.
// Or just a calculated version like:
// VERSION=1.1.0-rc20250129 cargo build

fn main() {
    // Read version from the environment variable
    let version = std::env::var("VERSION").unwrap_or_else(|_| "unknown".to_string());

    // Instruct Cargo to rerun build.rs if it changes
    println!("cargo:rerun-if-changed=build.rs");
    // Pass the version to the Rust code
    println!("cargo:rustc-env=version={}", version);
}
