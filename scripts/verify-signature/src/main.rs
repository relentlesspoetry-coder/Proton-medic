use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{env, fs};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 { return Err("Usage: verify-release tauri.conf.json installer.deb version".into()); }
    let config: serde_json::Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let key = config["plugins"]["updater"]["pubkey"].as_str().ok_or("Missing public key")?;
    let key = String::from_utf8(STANDARD.decode(key.trim())?)?;
    let key = PublicKey::decode(&key)?;
    let signature = fs::read_to_string(format!("{}.sig", args[2]))?;
    let decoded = String::from_utf8(STANDARD.decode(signature.trim())?)?;
    let expected = format!("version:{}", args[3]);
    if !decoded.lines().any(|line| line.starts_with("trusted comment:") && line.split_whitespace().any(|word| word == expected)) {
        return Err("Signature version does not match release".into());
    }
    let signature = Signature::decode(&decoded)?;
    let bytes = fs::read(&args[2])?;
    key.verify(&bytes, &signature, true)?;
    // The same key/signature must reject changed package bytes.
    let mut corrupted = bytes;
    if corrupted.is_empty() { return Err("Empty package".into()); }
    corrupted[0] ^= 1;
    if key.verify(&corrupted, &signature, true).is_ok() { return Err("Corruption check unexpectedly passed".into()); }
    println!("Signature and signed version verified; altered package rejected");
    Ok(())
}
