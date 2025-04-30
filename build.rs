/**
* filename : build
* author : HAMA
* date: 2025. 4. 30.
* description: 
**/

use std::env;
use std::process::Command;


fn main() {
  // 빌드 시간 표시
  println!("cargo:rerun-if-changed=src");
  println!("cargo:rerun-if-changed=build.rs");
  
  // 빌드 정보 수집
  let git_hash = get_git_hash();
  let build_date = chrono::Utc::now().to_rfc3339();
  let rust_version = env!("CARGO_PKG_RUST_VERSION", "unknown");
  
  // 환경 변수로 전달
  println!("cargo:rustc-env=BUILD_GIT_HASH={}", git_hash);
  println!("cargo:rustc-env=BUILD_DATE={}", build_date);
  println!("cargo:rustc-env=RUST_VERSION={}", rust_version);
}

fn get_git_hash() -> String {
  let output = Command::new("git")
    .args(&["rev-parse", "--short", "HEAD"])
    .output();
  
  match output {
    Ok(output) if output.status.success() => {
      String::from_utf8_lossy(&output.stdout).trim().to_string()
    }
    _ => "unknown".to_string(),
  }
}