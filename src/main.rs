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
    #[cfg(feature = "http")]
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "serve-http")
    {
        return run_http_cli();
    }
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
        #[cfg(feature = "http")]
        println!(
            "       zixcel-mcp-gateway serve-http --config <FILE> --listen <LOOPBACK_IP:PORT> --public-origin <HTTPS_ORIGIN> [--allow-origin <HTTPS_ORIGIN>]"
        );
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

#[cfg(feature = "http")]
fn run_http_cli() -> Result<(), String> {
    use zixcel_mcp_gateway::http::{HttpOptions, run_http};
    let args = std::env::args().skip(2).collect::<Vec<_>>();
    let mut file = None;
    let mut listen = None;
    let mut origin = None;
    let mut browser_origins = Vec::new();
    let mut pairs = args.chunks_exact(2);
    for pair in &mut pairs {
        match pair[0].as_str() {
            "--config" if file.is_none() => file = Some(PathBuf::from(&pair[1])),
            "--listen" if listen.is_none() => {
                listen = Some(
                    pair[1]
                        .parse::<std::net::SocketAddr>()
                        .map_err(|_| "An explicit loopback IP:port is required")?,
                );
            }
            "--public-origin" if origin.is_none() => origin = Some(pair[1].clone()),
            "--allow-origin" if browser_origins.len() < 7 => browser_origins.push(pair[1].clone()),
            _ => return Err("Unknown or duplicate HTTP option".into()),
        }
    }
    if !pairs.remainder().is_empty() {
        return Err("HTTP options require values".into());
    }
    let config = Config::load(&file.ok_or("--config is required")?)?;
    let address = listen.ok_or("--listen is required")?;
    if !address.ip().is_loopback() {
        return Err("HTTP listener must be loopback".into());
    }
    let mut options = HttpOptions::new(&origin.ok_or("--public-origin is required")?)?;
    for allowed in browser_origins {
        options = options.allow_browser_origin(&allowed)?;
    }
    tokio::runtime::Builder::new_current_thread().enable_all().build()
        .map_err(|_| "HTTP runtime unavailable")?.block_on(async {
            let listener = tokio::net::TcpListener::bind(address).await.map_err(|_| "Loopback socket unavailable")?;
            eprintln!("{}", serde_json::json!({"event":"listening", "address":listener.local_addr().map_err(|_| "Loopback socket unavailable")?.to_string(), "path":"/mcp", "external_actions":false}));
            run_http(Gateway::new(config), listener, options, async { let _ = tokio::signal::ctrl_c().await; }).await
        })
}
