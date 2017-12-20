#![feature(duration_from_micros)]

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_timer;
extern crate bytes;

use std::env;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use tokio_timer::*;

use tokio_core::reactor::{self, Core};

use futures::sync::mpsc;
use futures::{Sink, Future, Stream};
use futures::IntoFuture;

fn main() {

    let mut args = env::args().skip(1).collect::<Vec<_>>();
    let tcp = match args.iter().position(|a| a == "--udp") {
        Some(i) => {
            args.remove(i);
            false
        }
        None => true,
    };

    let addr = args.first().unwrap_or_else(|| {
        panic!("This program requires at least one argument")
    });
    let addr = addr.parse::<SocketAddr>().unwrap();
    const TICK_DURATION: u64 = 100;
    const TIMER_INTERVAL: u64 = 200;

    let MAX_TIME: Duration = Duration::from_secs(10);


    let mut core = Core::new().expect("Core");
    let handle = core.handle();

    let (mut msg_sender, msg_receiver) = mpsc::channel(0);
    let msg_receiver = msg_receiver.map_err(|_| panic!()); // errors not possible on rx

    let sender: Box<Future<Item=(), Error=io::Error>> = tcp::connect(&addr, core.handle(), Box::new(msg_receiver));

    let timer = tokio_timer::wheel().tick_duration(Duration::from_micros(TICK_DURATION)).build();
    let timer = timer.interval(Duration::from_micros(TIMER_INTERVAL)).for_each(move |_| {
        let msg = "hello world!!\n";
        println!("Sending: {}", msg);
        msg_sender.clone().send(msg.as_bytes().to_vec()).wait().unwrap();
        Ok(())
    });


    let timeout: Box<Future<Item=(), Error=io::Error>> = Box::new(reactor::Timeout::new(MAX_TIME, &handle)
        .into_future()
        .and_then(move |timeout| timeout.and_then(move |_| Ok(())))
        .map_err(|_| io::Error::new(io::ErrorKind::Other, format!("Error in timeout"))));

    let f = timeout.select(sender)
        .map_err(|e| panic!("Fail!!!"))
        .map(|_| ());

    core.handle().spawn(timer.map_err(|_| ()));

    core.run(f).unwrap();
}

mod tcp {
    use std::io::{self, Read, Write};
    use std::net::SocketAddr;

    use bytes::{BufMut, BytesMut};
    use futures::{Future, Stream};
    use futures::stream::{self, MapErr};
    use tokio_core::net::TcpStream;
    use tokio_core::reactor::Handle;
    use tokio_io;
    use tokio_io::codec::Framed;
    use tokio_io::AsyncRead;
    use tokio_io::codec::{Encoder, Decoder};
    use futures::IntoFuture;
    use futures::Sink;
    use futures::stream::*;
    use futures::sink::SendAll;

    use std;
    use std::time::Duration;
    use tokio_timer;
    use tokio_timer::*;

    pub fn connect(addr: &SocketAddr, handle: Handle,
            input_stream: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>) -> Box<Future<Item=(), Error=io::Error>> {

        let tcp = TcpStream::connect(&addr, &handle);

        let mut stdout = io::stdout();
        let client = tcp.and_then(|stream| {
            let (sink, stream) = stream.framed(Bytes).split();
            let send_stdin = input_stream.forward(sink);
            let write_stdout = stream.for_each(move |buf| {
                println!("Receiving: {:?}", buf.as_ref());
                Ok(())
            });
            send_stdin
                .map(|_| ())
                .then(|_| Ok(()))
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
            let size = buf.capacity();
            let size2 = buf.remaining_mut();
            buf.extend(data);
            Ok(())
        }
    }
}

//mod udp {
//    use std::io;
//    use std::net::SocketAddr;
//
//    use bytes::BytesMut;
//    use futures::{Future, Stream};
//    use futures::future::Executor;
//    use tokio_core::net::{UdpCodec, UdpSocket};
//    use tokio_core::reactor::Handle;
//
//    pub fn connect(&addr: &SocketAddr,
//                   handle: Handle,
//                   stdin: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>)
//                   -> Box<Stream<Item = BytesMut, Error = io::Error>>
//    {
//        // We'll bind our UDP socket to a local IP/port, but for now we
//        // basically let the OS pick both of those.
//        let addr_to_bind = if addr.ip().is_ipv4() {
//            "0.0.0.0:0".parse().unwrap()
//        } else {
//            "[::]:0".parse().unwrap()
//        };
//        let udp = UdpSocket::bind(&addr_to_bind, &handle)
//            .expect("failed to bind socket");
//
//        // Like above with TCP we use an instance of `UdpCodec` to transform
//        // this UDP socket into a framed sink/stream which operates over
//        // discrete values. In this case we're working with *pairs* of socket
//        // addresses and byte buffers.
//        let (sink, stream) = udp.framed(Bytes).split();
//
//        // All bytes from `stdin` will go to the `addr` specified in our
//        // argument list. Like with TCP this is spawned concurrently
//        pool.execute(stdin.map(move |chunk| {
//            (addr, chunk)
//        }).forward(sink).then(|result| {
//            if let Err(e) = result {
//                panic!("failed to write to socket: {}", e)
//            }
//            Ok(())
//        })).unwrap();
//
//        // With UDP we could receive data from any source, so filter out
//        // anything coming from a different address
//        Box::new(stream.filter_map(move |(src, chunk)| {
//            if src == addr {
//                Some(chunk.into())
//            } else {
//                None
//            }
//        }))
//    }
//
//    struct Bytes;
//
//    impl UdpCodec for Bytes {
//        type In = (SocketAddr, Vec<u8>);
//        type Out = (SocketAddr, Vec<u8>);
//
//        fn decode(&mut self, addr: &SocketAddr, buf: &[u8]) -> io::Result<Self::In> {
//            Ok((*addr, buf.to_vec()))
//        }
//
//        fn encode(&mut self, (addr, buf): Self::Out, into: &mut Vec<u8>) -> SocketAddr {
//            into.extend(buf);
//            addr
//        }
//    }
//}
