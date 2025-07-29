use std::env;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let profile = env::var("PROFILE").unwrap();
    
    println!("cargo:rerun-if-changed=../src");
    println!("cargo:rerun-if-changed=../proxy/src");
    println!("cargo:rerun-if-changed=../tasks/src");
    
    // Look for pre-built binaries in the workspace target directory
    let workspace_target = "../target";
    let build_profile = if profile == "release" { "release" } else { "debug" };
    
    let enigmatick_path = format!("{}/{}/enigmatick", workspace_target, build_profile);
    let proxy_path = format!("{}/{}/proxy", workspace_target, build_profile);
    let tasks_path = format!("{}/{}/tasks", workspace_target, build_profile);
    
    // Check if binaries exist, if not provide helpful error
    if !Path::new(&enigmatick_path).exists() {
        panic!("enigmatick binary not found at {}. Please run: cargo build --bin enigmatick", enigmatick_path);
    }
    
    if !Path::new(&proxy_path).exists() {
        panic!("proxy binary not found at {}. Please run: cd proxy && cargo build --target-dir ../target", proxy_path);
    }
    
    if !Path::new(&tasks_path).exists() {
        panic!("tasks binary not found at {}. Please run: cd tasks && cargo build --target-dir ../target", tasks_path);
    }
    
    // Copy binaries to OUT_DIR
    std::fs::copy(&enigmatick_path, format!("{}/enigmatick", out_dir))
        .expect("Failed to copy enigmatick binary");
    
    std::fs::copy(&proxy_path, format!("{}/proxy", out_dir))
        .expect("Failed to copy proxy binary");
    
    std::fs::copy(&tasks_path, format!("{}/tasks", out_dir))
        .expect("Failed to copy tasks binary");
    
    println!("cargo:warning=Successfully embedded pre-built binaries");
}
