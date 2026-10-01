//! Minimal executable wrapper; configuration is the only permitted file input.

use std::io::BufReader;
use std::path::PathBuf;
use zixcel_mcp_gateway::Config;
use zixcel_mcp_gateway::Gateway;
use zixcel_mcp_gateway::run_stdio;

enum Mode {
    Serve,
    Validate,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("zixcel-mcp-gateway: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let (mode, path) = parse_args()?;
    let config = Config::load(&path)?;
    if matches!(mode, Mode::Validate) {
        println!(
            "{}",
            serde_json::json!({
                "schema": "zixcel://mcp/public-catalog-validation/v1",
                "schema_version": config.schema_version,
                "server_id": config.server.id.as_str(),
                "resource_count": config.resources.len(),
                "valid": true,
                "network_access": false,
                "external_actions": false
            })
        );
        return Ok(());
    }
    let gateway = Gateway::new(config);
    run_stdio(
        &gateway,
        BufReader::new(std::io::stdin().lock()),
        std::io::stdout().lock(),
    )
}

fn parse_args() -> Result<(Mode, PathBuf), String> {
    let mut args = std::env::args_os();
    let _program = args.next();
    let Some(flag) = args.next() else {
        return Err("usage: zixcel-mcp-gateway --config <FILE>".into());
    };
    if flag == "--help" {
        println!("usage: zixcel-mcp-gateway [validate-config] --config <FILE>");
        std::process::exit(0);
    }
    if flag == "--version" {
        println!("zixcel-mcp-gateway {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let mode = if flag == "validate-config" {
        let config_flag = args.next().ok_or("validate-config requires --config")?;
        if config_flag != "--config" {
            return Err("validate-config requires --config <FILE>".into());
        }
        Mode::Validate
    } else if flag == "--config" {
        Mode::Serve
    } else {
        return Err("only [validate-config] --config <FILE> is accepted".into());
    };
    let path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "--config requires a file".to_string())?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    Ok((mode, path))
}
