//! Bounded newline-delimited stdio transport; stdout is reserved for JSON-RPC.

use crate::Gateway;
use std::io::BufRead;
use std::io::Write;

const MAX_REQUEST_BYTES: u64 = 1024 * 1024;

/// Serves newline-delimited JSON-RPC until stdin reaches EOF.
///
/// # Errors
///
/// Returns an error for oversized, non-UTF-8, unterminated, or failed I/O.
pub fn run_stdio<R: BufRead, W: Write>(
    gateway: &Gateway,
    mut input: R,
    mut output: W,
) -> Result<(), String> {
    loop {
        let mut frame = Vec::new();
        let mut bounded = std::io::Read::take(&mut input, MAX_REQUEST_BYTES + 2);
        let read = bounded
            .read_until(b'\n', &mut frame)
            .map_err(|error| format!("stdin read failed: {error}"))?;
        if read == 0 {
            return Ok(());
        }
        if frame.len() as u64 > MAX_REQUEST_BYTES || !frame.ends_with(b"\n") {
            return Err("JSON-RPC frame exceeds 1 MiB or lacks newline delimiter".into());
        }
        let line = std::str::from_utf8(&frame[..frame.len() - 1])
            .map_err(|_| "JSON-RPC frame is not UTF-8".to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = gateway.handle_line(line) {
            writeln!(output, "{response}")
                .map_err(|error| format!("stdout write failed: {error}"))?;
            output
                .flush()
                .map_err(|error| format!("stdout flush failed: {error}"))?;
        }
    }
}
