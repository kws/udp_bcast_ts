use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::process::ExitCode;
use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn usage(program: &str) -> String {
    format!(
        "Usage:
  {program} --addr <IPv4-or-IPv6> --port <1-65535> [--interval-ms <ms>]

Example:
  {program} --addr 255.255.255.255 --port 12321 --interval-ms 1000
"
    )
}

fn parse_u16(s: &str, flag: &str) -> Result<u16, String> {
    let v: u32 = s
        .parse()
        .map_err(|_| format!("Invalid value for {flag}: {s}"))?;
    if v == 0 || v > 65535 {
        return Err(format!("Port out of range for {flag}: {v}"));
    }
    Ok(v as u16)
}

fn parse_u64(s: &str, flag: &str) -> Result<u64, String> {
    s.parse()
        .map_err(|_| format!("Invalid value for {flag}: {s}"))
}

fn parse_ip(s: &str, flag: &str) -> Result<IpAddr, String> {
    s.parse()
        .map_err(|_| format!("Invalid IP address for {flag}: {s}"))
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
                let v = it.next().unwrap_or_default();
                if v.is_empty() {
                    eprintln!("Missing value for --addr\n{}", usage(&program));
                    return ExitCode::from(2);
                }
                match parse_ip(&v, "--addr") {
                    Ok(ip) => addr = Some(ip),
                    Err(e) => {
                        eprintln!("{e}\n{}", usage(&program));
                        return ExitCode::from(2);
                    }
                }
            }
            "--port" => {
                let v = it.next().unwrap_or_default();
                if v.is_empty() {
                    eprintln!("Missing value for --port\n{}", usage(&program));
                    return ExitCode::from(2);
                }
                match parse_u16(&v, "--port") {
                    Ok(p) => port = Some(p),
                    Err(e) => {
                        eprintln!("{e}\n{}", usage(&program));
                        return ExitCode::from(2);
                    }
                }
            }
            "--interval-ms" => {
                let v = it.next().unwrap_or_default();
                if v.is_empty() {
                    eprintln!("Missing value for --interval-ms\n{}", usage(&program));
                    return ExitCode::from(2);
                }
                match parse_u64(&v, "--interval-ms") {
                    Ok(ms) if ms > 0 => interval_ms = ms,
                    Ok(_) => {
                        eprintln!("--interval-ms must be > 0\n{}", usage(&program));
                        return ExitCode::from(2);
                    }
                    Err(e) => {
                        eprintln!("{e}\n{}", usage(&program));
                        return ExitCode::from(2);
                    }
                }
            }
            "-h" | "--help" => {
                print!("{}", usage(&program));
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("Unknown argument: {other}\n{}", usage(&program));
                return ExitCode::from(2);
            }
        }
    }

    let addr = match addr {
        Some(a) => a,
        None => {
            eprintln!("Missing required --addr\n{}", usage(&program));
            return ExitCode::from(2);
        }
    };
    let port = match port {
        Some(p) => p,
        None => {
            eprintln!("Missing required --port\n{}", usage(&program));
            return ExitCode::from(2);
        }
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
            return ExitCode::from(1);
        }
    };

    if let Err(e) = sock.set_broadcast(true) {
        eprintln!("Failed to enable broadcast: {e}");
        return ExitCode::from(1);
    }

    let dest = SocketAddr::new(addr, port);
    let interval = Duration::from_millis(interval_ms);

    loop {
        // milliseconds since Unix epoch
        let ts_ms: u64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_millis() as u64,
            Err(e) => {
                eprintln!("System clock error (before UNIX_EPOCH): {e:?}");
                return ExitCode::from(1);
            }
        };

        // 8-byte big-endian u64, equivalent to struct.pack("!Q", ts_ms)
        let payload = ts_ms.to_be_bytes();

        if let Err(e) = sock.send_to(&payload, dest) {
            eprintln!("send_to({dest}) failed: {e}");
            // Choose whether to exit or continue; for a service I'd typically continue.
        } else {
            println!("Sent broadcast to {dest} ts_ms={ts_ms}");
        }

        sleep(interval);
    }
}
