use std::io::Write;
use std::net::{SocketAddr, TcpStream, UdpSocket};

fn main() {
    // Minimal logging client example:
    // logging_client --address 127.0.0.1:8080 [--duration 10] [--tcp]
    pretty_env_logger::init();

    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    let tcp = extract_command_arg(&mut args, vec!["--tcp".into()]);
    let addr = extract_arg(
        &mut args,
        vec!["--address".into(), "-a".into()],
        None::<SocketAddr>,
        |x| x.parse::<SocketAddr>().unwrap(),
    );
    let duration = extract_arg(
        &mut args,
        vec!["--duration".into(), "-d".into()],
        Some(std::time::Duration::from_secs(5)),
        |x| std::time::Duration::from_secs(x.parse::<u64>().unwrap()),
    );

    let start = std::time::Instant::now();
    if tcp {
        let mut stream = TcpStream::connect(addr).expect("connect tcp");
        while start.elapsed() < duration {
            let _ = stream.write_all(b"hello world!!\n");
        }
    } else {
        let sock = UdpSocket::bind("127.0.0.1:0").expect("bind udp");
        while start.elapsed() < duration {
            let _ = sock.send_to(b"hello world!!\n", addr);
        }
    }
}

fn extract_arg<T>(
    args: &mut Vec<String>,
    names: Vec<String>,
    default: Option<T>,
    parser: fn(&String) -> T,
) -> T {
    match args.iter().position(|a| names.contains(a)) {
        Some(i) => {
            let value: T = parser(args.get(i + 1).unwrap());
            args.remove(i + 1);
            args.remove(i);
            value
        }
        None => default.unwrap_or_else(|| panic!("This parameter is required: {:?}", names)),
    }
}

fn extract_command_arg(args: &mut Vec<String>, names: Vec<String>) -> bool {
    match args.iter().position(|a| names.contains(a)) {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}
