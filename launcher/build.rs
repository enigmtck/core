use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();
    let target = env::var("TARGET").ok();

    println!("cargo:rerun-if-changed=../src");
    println!("cargo:rerun-if-changed=../proxy/src");
    println!("cargo:rerun-if-changed=../tasks/src");

    // Determine the workspace root (parent of launcher directory)
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get workspace root")
        .to_path_buf();

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let build_profile = if profile == "release" { "release" } else { "debug" };

    // Prepare target args and features for musl builds
    let mut target_args = vec![];
    let mut main_features = vec![];
    let mut proxy_features = vec![];

    if let Some(ref t) = target {
        target_args.push("--target");
        target_args.push(t.as_str());

        // For musl targets, enable vendored dependencies
        if t.contains("musl") {
            main_features.push("vendored-openssl");
            main_features.push("bundled-postgres");
            proxy_features.push("vendored-openssl");
        }
    }

    // Build main enigmatick binary
    println!("cargo:warning=Building enigmatick binary...");
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(&workspace_root)
        .args(&["build", "--bin", "enigmatick"]);
    if profile == "release" {
        cmd.arg("--release");
    }
    for arg in &target_args {
        cmd.arg(arg);
    }
    if !main_features.is_empty() {
        cmd.arg("--features");
        cmd.arg(main_features.join(","));
    }
    let status = cmd.status().expect("Failed to build enigmatick");

    if !status.success() {
        panic!("Failed to build enigmatick binary");
    }

    // Build proxy binary
    println!("cargo:warning=Building proxy binary...");
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(workspace_root.join("proxy"))
        .args(&["build"])
        .args(&["--target-dir", workspace_root.join("target").to_str().unwrap()]);
    if profile == "release" {
        cmd.arg("--release");
    }
    for arg in &target_args {
        cmd.arg(arg);
    }
    if !proxy_features.is_empty() {
        cmd.arg("--features");
        cmd.arg(proxy_features.join(","));
    }
    let status = cmd.status().expect("Failed to build proxy");

    if !status.success() {
        panic!("Failed to build proxy binary");
    }

    // Build tasks binary
    println!("cargo:warning=Building tasks binary...");
    let mut cmd = Command::new(&cargo);
    cmd.current_dir(workspace_root.join("tasks"))
        .args(&["build"])
        .args(&["--target-dir", workspace_root.join("target").to_str().unwrap()]);
    if profile == "release" {
        cmd.arg("--release");
    }
    for arg in &target_args {
        cmd.arg(arg);
    }
    let status = cmd.status().expect("Failed to build tasks");

    if !status.success() {
        panic!("Failed to build tasks binary");
    }

    // Copy binaries to OUT_DIR
    // If building for a custom target, binaries are in target/<TARGET>/<PROFILE>/
    // Otherwise they're in target/<PROFILE>/
    let target_dir = if let Some(ref t) = target {
        workspace_root.join("target").join(t).join(build_profile)
    } else {
        workspace_root.join("target").join(build_profile)
    };

    let enigmatick_path = target_dir.join("enigmatick");
    let proxy_path = target_dir.join("proxy");
    let tasks_path = target_dir.join("tasks");

    std::fs::copy(&enigmatick_path, format!("{}/enigmatick", out_dir))
        .expect("Failed to copy enigmatick binary");

    std::fs::copy(&proxy_path, format!("{}/proxy", out_dir))
        .expect("Failed to copy proxy binary");

    std::fs::copy(&tasks_path, format!("{}/tasks", out_dir))
        .expect("Failed to copy tasks binary");

    println!("cargo:warning=Successfully built and embedded all binaries");
}
