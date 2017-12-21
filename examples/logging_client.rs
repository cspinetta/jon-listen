
#[macro_use]
extern crate log;
extern crate pretty_env_logger;

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_timer;
extern crate bytes;

use std::env;
use std::vec::Vec;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio_core::reactor::{self, Core};

use futures::sync::mpsc;
use futures::stream;
use futures::{Sink, Future, Stream, future};
use futures::IntoFuture;

fn main() {

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let udp = extract_command_arg(args.as_mut(), vec!["--udp".to_string()]);
    let threads = extract_arg(args.as_mut(), vec!["--threads".to_string(), "-t".to_string()], Option::Some(10), |x| x.parse::<usize>().unwrap());
    let addr = extract_arg(args.as_mut(), vec!["--address".to_string(), "-a".to_string()], Option::None, |x| x.parse::<SocketAddr>().unwrap());
    let exec_duration = extract_arg(args.as_mut(), vec!["--duration".to_string(), "-d".to_string()], Option::Some(Duration::from_secs(10)),
                                    |x| { Duration::from_secs(x.parse::<u64>().unwrap()) });

    let mut core = Core::new().expect("Creating event loop");
    let handle = core.handle();

    let (msg_sender, msg_receiver) = mpsc::channel(0);
    let msg_receiver = msg_receiver.map_err(|_| panic!("Error in rx")); // errors not possible on rx

    let stream = stream::repeat("hello world!!\n".as_bytes().to_vec());
    let generator = msg_sender.send_all(stream);
    let generator = generator
        .then(move |res| {
            if let Err(e) = res {
                println!("Occur an error generating messages: {:?}", e);
                ()
            }
            Ok(())
        });

    core.handle().spawn(generator);

    let sender: Box<Future<Item=(), Error=io::Error>> = if udp {
        println!("Starting UDP client");
        udp::connect(&addr, core.handle(), Box::new(msg_receiver))
    } else {
        println!("Starting TCP client");
        tcp::connect(&addr, core.handle(), Box::new(msg_receiver))
    };

    let timeout = reactor::Timeout::new(exec_duration, &handle)
        .into_future()
        .and_then(|timeout| timeout.and_then(move |_| {
            Ok(())
        }))
        .map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Error performing timeout: {:?}", e))
        });

    let f = timeout
        .select(sender)
        .then(|res| -> Box<Future<Item=_, Error=_>> {
               match res {
                   Ok((_, _)) => Box::new(future::ok(())),
                   Err((error, _)) => Box::new(future::err(error)),
               }
           });

//    let f = timeout.select(sender)
//        .map_err(|(a, _)| panic!("Fail!!!, {:?}", a))
//        .map(|_| ());

    core.run(f).expect("Event loop running");
}

fn extract_arg<T>(args: &mut Vec<String>, names: Vec<String>, default: Option<T>, parser: fn(&String) -> T) -> T {
    match args.iter().position(|a| names.contains(a)) {
        Some(i) => {
            let value: T = parser(args.get(i + 1).unwrap());
            args.remove(i + 1);
            args.remove(i);
            value
        }
        None => default.expect(format!("This parameter is required: {:?}", names).as_ref()),
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

mod tcp {
    use std::io;
    use std::net::SocketAddr;

    use bytes::BytesMut;
    use futures::{Future, Stream};
    use tokio_core::net::TcpStream;
    use tokio_core::reactor::Handle;
    use tokio_io::AsyncRead;
    use tokio_io::codec::{Encoder, Decoder};


    pub fn connect(addr: &SocketAddr, handle: Handle,
            input_stream: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>) -> Box<Future<Item=(), Error=io::Error>> {

        let tcp = TcpStream::connect(&addr, &handle);

        let client = tcp.and_then(|stream| {
            let (sink, _stream) = stream.framed(Bytes).split();
            let send_stdin = input_stream.forward(sink);
//            let write_stdout = stream.for_each(move |buf| {
//                println!("Receiving: {:?}", buf.as_ref());
//                Ok(())
//            });
            send_stdin.then(|_| Ok(()))
        });

        Box::new(client)
    }

    struct Bytes;

    impl Decoder for Bytes {
        type Item = BytesMut;
        type Error = io::Error;

        fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<BytesMut>> {
            if buf.len() > 0 {
                let len = buf.len();
                Ok(Some(buf.split_to(len)))
            } else {
                Ok(None)
            }
        }

        fn decode_eof(&mut self, buf: &mut BytesMut) -> io::Result<Option<BytesMut>> {
            self.decode(buf)
        }
    }

    impl Encoder for Bytes {
        type Item = Vec<u8>;
        type Error = io::Error;

        fn encode(&mut self, data: Vec<u8>, buf: &mut BytesMut) -> io::Result<()> {
            buf.extend(data);
            Ok(())
        }
    }
}

mod udp {
    use std::io;
    use std::net::SocketAddr;

    use futures::{Future, Stream};
    use tokio_core::net::{UdpCodec, UdpSocket};
    use tokio_core::reactor::Handle;


    pub fn connect(addr: &SocketAddr, handle: Handle,
            input_stream: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>) -> Box<Future<Item=(), Error=io::Error>> {
        let client_addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let udp = UdpSocket::bind(&client_addr, &handle).expect("Failed to bind client UDP socket");

        let (sink, stream) = udp.framed(Bytes).split();

        let addr = addr.clone();
        Box::new(input_stream
            .map(move |chunk| (addr, chunk))
            .forward(sink)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Occur an error sending datagrams: {:?}", e)))
            .map(|_| ()))
    }


    struct Bytes;

    impl UdpCodec for Bytes {
        type In = (SocketAddr, Vec<u8>);
        type Out = (SocketAddr, Vec<u8>);

        fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
            Ok((*addr, buf.to_vec()))
        }

        fn encode(&mut self, (addr, buf): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
            into.extend(buf);
            addr
        }
    }
}


//    const TICK_DURATION: u64 = 100;
//    const TIMER_INTERVAL: u64 = 200;

//    let timer = tokio_timer::wheel().tick_duration(Duration::from_micros(TICK_DURATION)).build();
//    let timer = timer.interval(Duration::from_micros(TIMER_INTERVAL)).for_each(move |_| {
//        let msg = "hello world!!\n";
//        println!("Sending: {}", msg);
//        msg_sender.clone().send(msg.as_bytes().to_vec()).wait().unwrap();
//        Ok(())
//    });
