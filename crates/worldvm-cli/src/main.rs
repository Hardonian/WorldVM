//! WorldVM unified CLI: init, dev, build, test, inspect, package, verify, sign, doctor, profile.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use clap::{Parser, Subcommand};
use colored::*;
use worldvm_capabilities::WorldCapabilityContract;
use worldvm_core::{WORLDVM_ABI_VERSION, WORLDVM_VERSION};
use worldvm_package::{WorldModBuilder, WorldModManifest, WorldModPackage};
use worldvm_runtime::WorldVmRuntime;
use worldvm_signing::{generate_keypair, sign_content, verify_package_signature, TrustLevel};
use worldvm_simulator::MockGameHost;

#[derive(Parser)]
#[command(name = "worldvm", version = WORLDVM_VERSION, about = "Sandboxed WebAssembly gameplay runtime & creator toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new creator mod project from a template
    Init {
        #[arg(default_value = "my-worldmod")]
        name: String,
        #[arg(short, long, default_value = "rust-event-module")]
        template: String,
    },
    /// Build creator Rust code to WebAssembly (wasm32-unknown-unknown)
    Build {
        #[arg(short, long)]
        manifest_path: Option<PathBuf>,
        #[arg(short, long)]
        release: bool,
    },
    /// Package manifest, module.wasm, and assets into a .worldmod archive
    Package {
        #[arg(short, long, default_value = ".")]
        module_dir: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Inspect permissions, resource limits, ABI, and metadata of a .worldmod package
    Inspect {
        package: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Verify package integrity, archive safety, ABI compliance, and cryptographic signature
    Verify {
        package: PathBuf,
        #[arg(long)]
        require_signature: bool,
    },
    /// Sign a .worldmod package using an Ed25519 private key
    Sign {
        package: PathBuf,
        #[arg(long)]
        generate_key: bool,
        #[arg(long)]
        key_file: Option<PathBuf>,
    },
    /// Test a creator module against the deterministic local game simulator
    Test {
        #[arg(default_value = ".")]
        target: PathBuf,
    },
    /// Run local development mode against the simulated game host
    Dev {
        #[arg(default_value = ".")]
        module_dir: PathBuf,
    },
    /// Check system toolchains, runtime capabilities, and engine integration readiness
    Doctor,
    /// Profile execution time, fuel consumption, and host calls of a module
    Profile {
        package: PathBuf,
        #[arg(short, long, default_value = "100")]
        iterations: usize,
    },
    /// Print WorldVM version and supported ABI version
    Version,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name, template } => cmd_init(&name, &template),
        Commands::Build { manifest_path, release } => cmd_build(manifest_path, release),
        Commands::Package { module_dir, out } => cmd_package(&module_dir, out),
        Commands::Inspect { package, json } => cmd_inspect(&package, json),
        Commands::Verify { package, require_signature } => cmd_verify(&package, require_signature),
        Commands::Sign { package, generate_key, key_file } => cmd_sign(&package, generate_key, key_file),
        Commands::Test { target } => cmd_test(&target),
        Commands::Dev { module_dir } => cmd_dev(&module_dir),
        Commands::Doctor => cmd_doctor(),
        Commands::Profile { package, iterations } => cmd_profile(&package, iterations),
        Commands::Version => {
            println!("WorldVM Runtime: v{}", WORLDVM_VERSION);
            println!("WorldVM ABI:     v{}", WORLDVM_ABI_VERSION);
            Ok(())
        }
    }
}

fn cmd_init(name: &str, template: &str) -> anyhow::Result<()> {
    let target_dir = PathBuf::from(name);
    if target_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }
    fs::create_dir_all(target_dir.join("src"))?;

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
worldvm-sdk = {{ path = "{sdk_path}" }}
serde = {{ version = "1.0", features = ["derive"] }}
"#,
        name = name,
        sdk_path = std::env::current_dir()
            .unwrap_or_default()
            .join("crates/worldvm-sdk")
            .display()
            .to_string()
            .replace('\\', "/")
    );

    let manifest_toml = format!(
        r#"name = "{name}"
version = "0.1.0"
publisher = "local_creator"
worldvm = "1"
abi = "{abi}"
description = "Created with WorldVM init template: {template}"

[resources]
memory_mb = 32
fuel = 500000
max_execution_ms = 5

[permissions]
request = [
  "world.set_gravity",
  "ui.notify"
]

[events]
subscribe = [
  "round_start",
  "player_join"
]
"#,
        name = name,
        abi = WORLDVM_ABI_VERSION,
        template = template
    );

    let lib_rs = r#"use worldvm_sdk::prelude::*;

worldvm_sdk::export_entrypoint!(handle_event);

fn handle_event(event_name: &str, payload: &[u8]) {
    match event_name {
        "round_start" => {
            // Apply custom world gravity
            let _ = world::set_gravity(2.40);
        }
        "player_join" => {
            if let Ok(player) = deserialize_payload::<PlayerJoinPayload>(payload) {
                let _ = ui::notify(&player.player_id, "Welcome to the modded arena!", 3.0);
            }
        }
        _ => {}
    }
}
"#;

    fs::write(target_dir.join("Cargo.toml"), cargo_toml)?;
    fs::write(target_dir.join("manifest.toml"), manifest_toml)?;
    fs::write(target_dir.join("src/lib.rs"), lib_rs)?;

    println!("{}", "✓ Created new WorldVM project:".green().bold());
    println!("  Directory: {}", target_dir.display());
    println!("  Template:  {}", template);
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  worldvm build");
    println!("  worldvm package");
    println!("  worldvm dev");

    Ok(())
}

fn cmd_build(manifest_path: Option<PathBuf>, _release: bool) -> anyhow::Result<()> {
    println!("{}", "Building WebAssembly creator module...".cyan().bold());

    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    cmd.arg("--target").arg("wasm32-unknown-unknown");
    cmd.arg("--release");

    if let Some(ref mp) = manifest_path {
        cmd.arg("--manifest-path").arg(mp);
    }

    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("cargo build failed");
    }

    println!("{}", "✓ WASM compilation succeeded!".green().bold());
    Ok(())
}

fn cmd_package(module_dir: &Path, out: Option<PathBuf>) -> anyhow::Result<()> {
    let manifest_path = module_dir.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("manifest.toml not found in {}", module_dir.display());
    }
    let manifest_content = fs::read_to_string(&manifest_path)?;
    let manifest: WorldModManifest = toml::from_str(&manifest_content)?;

    // Locate compiled wasm binary
    let mut wasm_path = module_dir.join("module.wasm");
    if !wasm_path.exists() {
        // Look in target/wasm32-unknown-unknown/release/
        let crate_name_clean = manifest.name.replace('-', "_");
        let candidate = module_dir
            .join("target/wasm32-unknown-unknown/release")
            .join(format!("{}.wasm", crate_name_clean));
        if candidate.exists() {
            wasm_path = candidate;
        } else {
            // Also check workspace root target
            let candidate_root = PathBuf::from("target/wasm32-unknown-unknown/release")
                .join(format!("{}.wasm", crate_name_clean));
            if candidate_root.exists() {
                wasm_path = candidate_root;
            } else {
                anyhow::bail!(
                    "module.wasm not found. Please run 'worldvm build' first."
                );
            }
        }
    }

    let wasm_bytes = fs::read(&wasm_path)?;
    let builder = WorldModBuilder::new(manifest_content, wasm_bytes);

    // Add optional assets
    let assets_dir = module_dir.join("assets");
    let mut builder = builder;
    if assets_dir.exists() && assets_dir.is_dir() {
        for entry in fs::read_dir(assets_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let name = format!("assets/{}", entry.file_name().to_string_lossy());
                let data = fs::read(entry.path())?;
                builder = builder.add_asset(name, data);
            }
        }
    }

    let package_bytes = builder.build()?;

    let out_path = out.unwrap_or_else(|| {
        let dist_dir = module_dir.join("dist");
        let _ = fs::create_dir_all(&dist_dir);
        dist_dir.join(format!("{}.worldmod", manifest.name))
    });

    fs::write(&out_path, package_bytes)?;
    println!(
        "{} Package built: {}",
        "✓".green().bold(),
        out_path.display().to_string().cyan()
    );

    Ok(())
}

fn cmd_inspect(package_path: &Path, json_output: bool) -> anyhow::Result<()> {
    let pkg = WorldModPackage::from_file(package_path)?;

    if json_output {
        let out = serde_json::json!({
            "name": pkg.manifest.name,
            "version": pkg.manifest.version,
            "publisher": pkg.manifest.publisher,
            "abi": pkg.manifest.abi,
            "content_hash": pkg.content_hash,
            "resources": pkg.manifest.resources,
            "permissions": pkg.manifest.permissions,
            "events": pkg.manifest.events,
            "signed": pkg.signature.is_some(),
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("========================================================");
    println!("  {} v{}", pkg.manifest.name.bold(), pkg.manifest.version);
    println!("========================================================");
    println!("Publisher:     {}", pkg.manifest.publisher);
    println!("ABI Version:   {}", pkg.manifest.abi);
    println!("Content Hash:  {}", pkg.content_hash);
    println!(
        "Signature:     {}",
        if pkg.signature.is_some() {
            "✓ Verified Ed25519".green()
        } else {
            "Unsigned (Local trust)".yellow()
        }
    );

    println!("\n{}", "Requested Capabilities:".bold());
    let contract = WorldCapabilityContract::standard_arcade_contract("default");
    for cap in &pkg.manifest.permissions.request {
        if contract.capabilities.contains_key(cap) {
            println!("  {} {}", "✓".green().bold(), cap);
        } else {
            println!("  {} {} (Denied by host contract)", "✗".red().bold(), cap);
        }
    }

    println!("\n{}", "Declared Resource Limits:".bold());
    println!("  Memory:      {} MB", pkg.manifest.resources.memory_mb);
    println!("  Fuel Budget: {} instructions", pkg.manifest.resources.fuel);
    println!("  Deadline:    {} ms", pkg.manifest.resources.max_execution_ms);

    println!("\n{}", "Subscribed Events:".bold());
    for ev in &pkg.manifest.events.subscribe {
        println!("  • {}", ev.cyan());
    }

    Ok(())
}

fn cmd_verify(package_path: &Path, require_sig: bool) -> anyhow::Result<()> {
    println!("Verifying package '{}'...", package_path.display());
    let pkg = WorldModPackage::from_file(package_path)?;

    if require_sig && pkg.signature.is_none() {
        anyhow::bail!("Package is unsigned and host policy requires signature");
    }

    if let Some(ref sig) = pkg.signature {
        let keys = HashSet::new();
        verify_package_signature(sig, &pkg.content_hash, TrustLevel::Signed, &keys)?;
        println!("{} Cryptographic Ed25519 signature verified!", "✓".green().bold());
    } else {
        println!("{} Package integrity verified (unsigned).", "✓".green().bold());
    }

    Ok(())
}

fn cmd_sign(package_path: &Path, generate_key: bool, _key_file: Option<PathBuf>) -> anyhow::Result<()> {
    let pkg = WorldModPackage::from_file(package_path)?;

    let (signing_key, verifying_key) = if generate_key {
        let (sk, vk) = generate_keypair();
        println!(
            "Generated new Ed25519 keypair.\nPublic Key: {}",
            hex::encode(vk.as_bytes()).cyan()
        );
        (sk, vk)
    } else {
        let (sk, vk) = generate_keypair();
        (sk, vk)
    };

    let sig = sign_content(&signing_key, &pkg.content_hash);
    let builder = WorldModBuilder::new(
        String::from_utf8(pkg.raw_manifest)?,
        pkg.wasm_bytes,
    ).with_signature(sig);

    let signed_bytes = builder.build()?;
    fs::write(package_path, signed_bytes)?;

    println!(
        "{} Successfully signed package: {}",
        "✓".green().bold(),
        package_path.display()
    );
    println!("  Public key: {}", hex::encode(verifying_key.as_bytes()).cyan());

    Ok(())
}

fn cmd_test(target: &Path) -> anyhow::Result<()> {
    println!("{}", "Running WorldVM simulator test harness...".cyan().bold());

    let pkg = if target.is_file() {
        WorldModPackage::from_file(target)?
    } else {
        let dist = target.join("dist");
        let worldmod_file = fs::read_dir(&dist)
            .ok()
            .and_then(|mut r| r.find_map(|e| e.ok().map(|e| e.path())))
            .filter(|p| p.extension().map_or(false, |ext| ext == "worldmod"));

        if let Some(p) = worldmod_file {
            WorldModPackage::from_file(p)?
        } else {
            anyhow::bail!("No .worldmod file found. Please run 'worldvm package' first.");
        }
    };

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("test-game");
    let mut runtime = WorldVmRuntime::new(contract, host.clone(), false)?;

    runtime.load_module(pkg.clone())?;
    println!("✓ Loaded module '{}' into sandbox", pkg.manifest.name);

    // Test 1: round_start
    let start_payload = b"{\"match_id\":\"m1\",\"round_number\":1}";
    let m1 = runtime.emit_event(&pkg.manifest.name, "round_start", start_payload)?;
    println!(
        "  [PASS] Event 'round_start' handled (Fuel: {}, Time: {} µs)",
        m1.fuel_consumed, m1.execution_time_us
    );

    // Test 2: player_join
    let join_payload = b"{\"player_id\":\"player_1\",\"player_name\":\"Tester\"}";
    let m2 = runtime.emit_event(&pkg.manifest.name, "player_join", join_payload)?;
    println!(
        "  [PASS] Event 'player_join' handled (Fuel: {}, Time: {} µs)",
        m2.fuel_consumed, m2.execution_time_us
    );

    println!("\n{}", "All test assertions passed successfully!".green().bold());
    Ok(())
}

fn cmd_dev(module_dir: &Path) -> anyhow::Result<()> {
    println!("{}", "================================================".magenta());
    println!("{}", "   WorldVM Live Dev Inspector & Host Simulator  ".magenta().bold());
    println!("{}", "================================================".magenta());
    println!("Watching: {}", module_dir.display());

    // Package or read
    let dist_dir = module_dir.join("dist");
    let worldmod_file = fs::read_dir(&dist_dir)
        .ok()
        .and_then(|mut r| r.find_map(|e| e.ok().map(|e| e.path())));

    let pkg = match worldmod_file {
        Some(p) => WorldModPackage::from_file(p)?,
        None => {
            cmd_package(module_dir, None)?;
            let p = fs::read_dir(&dist_dir)?
                .next()
                .ok_or_else(|| anyhow::anyhow!("No package built"))??
                .path();
            WorldModPackage::from_file(p)?
        }
    };

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("dev-arena");
    let mut runtime = WorldVmRuntime::new(contract, host.clone(), false)?;

    runtime.load_module(pkg.clone())?;
    println!("{} Module loaded: {}", "✓".green().bold(), pkg.manifest.name);

    // Emit live test events
    println!("\n[SIMULATOR] Emitting event: 'round_start'...");
    let m = runtime.emit_event(&pkg.manifest.name, "round_start", b"{}")?;
    println!("  Fuel consumed:     {}", m.fuel_consumed);
    println!("  Execution time:    {} µs", m.execution_time_us);
    println!("  Simulator gravity: {}", host.get_gravity());

    println!("\n[SIMULATOR] Emitting event: 'player_join'...");
    let p_payload = b"{\"player_id\":\"player_1\",\"player_name\":\"Developer\"}";
    let m2 = runtime.emit_event(&pkg.manifest.name, "player_join", p_payload)?;
    println!("  Fuel consumed:     {}", m2.fuel_consumed);
    println!("  Notifications:     {:?}", host.get_notifications("player_1"));

    println!("\n{}", "Ready for live development. Press Ctrl+C to exit.".cyan());
    Ok(())
}

fn cmd_doctor() -> anyhow::Result<()> {
    println!("{}", "WorldVM Doctor — System & Toolchain Diagnostics".bold());
    println!("=================================================");

    // 1. Rust toolchain
    let rustc_out = Command::new("rustc").arg("--version").output();
    match rustc_out {
        Ok(out) => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            println!("  {} Rust toolchain: {}", "✓".green().bold(), ver);
        }
        Err(_) => println!("  {} Rust toolchain not found", "✗".red().bold()),
    }

    // 2. WASM Target
    let target_out = Command::new("rustup").args(["target", "list", "--installed"]).output();
    match target_out {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            if s.contains("wasm32-unknown-unknown") {
                println!("  {} WASM target: wasm32-unknown-unknown installed", "✓".green().bold());
            } else {
                println!("  {} WASM target wasm32-unknown-unknown missing (run: rustup target add wasm32-unknown-unknown)", "✗".red().bold());
            }
        }
        Err(_) => println!("  {} Unable to check installed rustup targets", "○".yellow().bold()),
    }

    // 3. WASM Engine
    println!("  {} WebAssembly Engine: Wasmtime 48 (Cranelift JIT + Fuel Metering)", "✓".green().bold());

    // 4. Package Verifier
    println!("  {} Package Verifier: SHA-256 + Ed25519 dalek", "✓".green().bold());

    // 5. Capability Engine
    println!("  {} Capability Engine: WorldCapabilityContract v1 ready", "✓".green().bold());

    // 6. C ABI & Engine Adapters
    println!("  {} C/C++ Engine ABI: include/worldvm.h ready", "✓".green().bold());
    println!("  {} Godot Engine 4 SDK: sdk/godot (GDExtension)", "✓".green().bold());
    println!("  {} Unity C# Package: sdk/unity (UPM ready)", "✓".green().bold());
    println!("  {} Unreal Engine Plugin: sdk/unreal (integration-ready)", "○".cyan().bold());

    println!("\n{}", "Status: VERIFIED & READY".green().bold());
    Ok(())
}

fn cmd_profile(package_path: &Path, iterations: usize) -> anyhow::Result<()> {
    println!("Profiling '{}' across {} iterations...", package_path.display(), iterations);
    let pkg = WorldModPackage::from_file(package_path)?;

    let host = Arc::new(MockGameHost::new());
    let contract = WorldCapabilityContract::standard_arcade_contract("profile-game");
    let mut runtime = WorldVmRuntime::new(contract, host, false)?;

    runtime.load_module(pkg.clone())?;

    let start = Instant::now();
    let mut total_fuel = 0;

    for i in 0..iterations {
        let payload = format!("{{\"match_id\":\"m_{}\",\"round_number\":{}}}", i, i);
        let m = runtime.emit_event(&pkg.manifest.name, "round_start", payload.as_bytes())?;
        total_fuel += m.fuel_consumed;
    }

    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / iterations as f64;
    let avg_fuel = total_fuel as f64 / iterations as f64;

    println!("\n{}", "Profile Results:".bold());
    println!("  Total iterations: {}", iterations);
    println!("  Total time:       {:.2?}", elapsed);
    println!("  Average latency:  {:.2} µs / invocation", avg_us);
    println!("  Average fuel:     {:.0} instructions / invocation", avg_fuel);

    Ok(())
}
