use std::convert::TryInto;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const EXIT_CODE_USAGE_ERROR: u8 = 2;
const EXIT_CODE_RUNTIME_ERROR: u8 = 1;

/// Returns the usage message for the program.
fn usage(program: &str) -> String {
    format!(
        "Usage:
  {program} --addr <IPv4-or-IPv6> --port <1-65535> [--interval-ms <ms>]

Example:
  {program} --addr 255.255.255.255 --port 12321 --interval-ms 1000
  {program} --addr ff02::1 --port 12321 --interval-ms 500
"
    )
}

/// Parses a string as a u16 port number (1-65535).
fn parse_u16(s: &str, flag: &str) -> Result<u16, String> {
    let v: u32 = s
        .parse()
        .map_err(|_| format!("Invalid value for {flag}: {s}"))?;
    if v == 0 || v > 65535 {
        return Err(format!("Port out of range for {flag}: {v}"));
    }
    Ok(v as u16)
}

/// Parses a string as a u64 value.
fn parse_u64(s: &str, flag: &str) -> Result<u64, String> {
    s.parse()
        .map_err(|_| format!("Invalid value for {flag}: {s}"))
}

/// Parses a string as an IP address (IPv4 or IPv6).
fn parse_ip(s: &str, flag: &str) -> Result<IpAddr, String> {
    s.parse()
        .map_err(|_| format!("Invalid IP address for {flag}: {s}"))
}

/// Helper function to get the next argument value or return an error.
fn get_arg_value(
    it: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<String, String> {
    match it.next() {
        Some(v) if !v.is_empty() => Ok(v),
        _ => Err(format!("Missing value for {flag}")),
    }
}

/// Helper function to print an error and return exit code.
fn error_exit(msg: &str, program: &str, code: u8) -> ExitCode {
    eprintln!("{msg}\n{}", usage(program));
    ExitCode::from(code)
}

fn main() -> ExitCode {
    let program = env::args().next().unwrap_or_else(|| "udp_bcast_ts".to_string());

    let mut addr: Option<IpAddr> = None;
    let mut port: Option<u16> = None;
    let mut interval_ms: u64 = 1000;

    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--addr" => {
                let v = match get_arg_value(&mut it, "--addr") {
                    Ok(v) => v,
                    Err(e) => return error_exit(&e, &program, EXIT_CODE_USAGE_ERROR),
                };
                match parse_ip(&v, "--addr") {
                    Ok(ip) => addr = Some(ip),
                    Err(e) => return error_exit(&e, &program, EXIT_CODE_USAGE_ERROR),
                }
            }
            "--port" => {
                let v = match get_arg_value(&mut it, "--port") {
                    Ok(v) => v,
                    Err(e) => return error_exit(&e, &program, EXIT_CODE_USAGE_ERROR),
                };
                match parse_u16(&v, "--port") {
                    Ok(p) => port = Some(p),
                    Err(e) => return error_exit(&e, &program, EXIT_CODE_USAGE_ERROR),
                }
            }
            "--interval-ms" => {
                let v = match get_arg_value(&mut it, "--interval-ms") {
                    Ok(v) => v,
                    Err(e) => return error_exit(&e, &program, EXIT_CODE_USAGE_ERROR),
                };
                match parse_u64(&v, "--interval-ms") {
                    Ok(ms) if ms > 0 => interval_ms = ms,
                    Ok(_) => {
                        return error_exit(
                            "--interval-ms must be > 0",
                            &program,
                            EXIT_CODE_USAGE_ERROR,
                        );
                    }
                    Err(e) => return error_exit(&e, &program, EXIT_CODE_USAGE_ERROR),
                }
            }
            "-h" | "--help" => {
                print!("{}", usage(&program));
                return ExitCode::SUCCESS;
            }
            other => {
                return error_exit(
                    &format!("Unknown argument: {other}"),
                    &program,
                    EXIT_CODE_USAGE_ERROR,
                );
            }
        }
    }

    let addr = match addr {
        Some(a) => a,
        None => return error_exit("Missing required --addr", &program, EXIT_CODE_USAGE_ERROR),
    };
    let port = match port {
        Some(p) => p,
        None => return error_exit("Missing required --port", &program, EXIT_CODE_USAGE_ERROR),
    };

    // Bind to an ephemeral local port on the appropriate address family.
    // (This avoids having to know the local interface address.)
    let bind_addr = match addr {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED), 0),
    };

    let sock = match UdpSocket::bind(bind_addr) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind UDP socket on {bind_addr}: {e}");
            return ExitCode::from(EXIT_CODE_RUNTIME_ERROR);
        }
    };

    if let Err(e) = sock.set_broadcast(true) {
        eprintln!("Failed to enable broadcast: {e}");
        return ExitCode::from(EXIT_CODE_RUNTIME_ERROR);
    }

    let dest = SocketAddr::new(addr, port);
    let interval = Duration::from_millis(interval_ms);

    loop {
        // Get milliseconds since Unix epoch
        let ts_ms: u64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => {
                // Convert u128 to u64, checking for overflow
                match d.as_millis().try_into() {
                    Ok(ms) => ms,
                    Err(_) => {
                        eprintln!("Timestamp overflow: system time too large for u64");
                        return ExitCode::from(EXIT_CODE_RUNTIME_ERROR);
                    }
                }
            }
            Err(e) => {
                eprintln!("System clock error (before UNIX_EPOCH): {e:?}");
                return ExitCode::from(EXIT_CODE_RUNTIME_ERROR);
            }
        };

        // 8-byte big-endian u64, equivalent to struct.pack("!Q", ts_ms)
        let payload = ts_ms.to_be_bytes();

        match sock.send_to(&payload, dest) {
            Ok(_) => {
                println!("Sent broadcast to {dest} ts_ms={ts_ms}");
            }
            Err(e) => {
                eprintln!("send_to({dest}) failed: {e}");
                // Continue on send errors to allow recovery from transient network issues
            }
        }

        sleep(interval);
    }
}
