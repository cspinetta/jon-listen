use std::net::UdpSocket;
use std::thread;
use std::time::Duration;

fn main() {
    // Minimal example: send many UDP messages to an address.
    let addr = std::env::args().nth(1).unwrap_or("127.0.0.1:8080".into());
    let count: usize = std::env::args()
        .nth(2)
        .unwrap_or("1000".into())
        .parse()
        .unwrap_or(1000);

    let any = "127.0.0.1:0";
    let client = UdpSocket::bind(any).expect("bind udp");
    thread::sleep(Duration::from_millis(1));

    for i in 1..=count {
        let msg = format!("Message # {}\n", i);
        let _ = client.send_to(msg.as_bytes(), &addr).expect("send_to");
    }
}
