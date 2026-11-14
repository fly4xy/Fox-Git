/// Passes GitHub OAuth credentials from the project's `.env` to `option_env!`.
fn main() {
    println!("cargo:rerun-if-changed=../.env");
    for name in ["FOXGIT_GITHUB_CLIENT_ID", "FOXGIT_GITHUB_CLIENT_SECRET"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if let Ok(env) = std::fs::read_to_string("../.env") {
        for line in env.lines() {
            if let Some((key, value)) = line.split_once('=') {
                if key.starts_with("FOXGIT_GITHUB_") && std::env::var_os(key).is_none() {
                    println!("cargo:rustc-env={key}={}", value.trim());
                }
            }
        }
    }
    tauri_build::build()
}
