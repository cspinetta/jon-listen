
use std::net::SocketAddr;
use std::thread::{self, JoinHandle};
use std::sync::Arc;

use tokio_core::reactor::Core;

use tokio_core::net::TcpListener;

use net2;
use net2::unix::UnixTcpBuilderExt;

use settings::Settings;

use futures::future::{self, FutureResult};
use futures::{Stream, Sink, Future};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Framed, Encoder, Decoder};

use bytes::BytesMut;
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;
use tokio_service::NewService;

use ::writer::file_writer::FileWriterCommand;
use std::sync::mpsc::{SyncSender, SendError};

use std::io;
use std::str;
use std::borrow::Borrow;

pub struct TcpServer;

impl TcpServer {

    pub fn start(settings: Arc<Settings>, sender: SyncSender<FileWriterCommand>) -> Vec<JoinHandle<()>> {

        let addr = format!("{}:{}", settings.server.host, settings.server.port).parse::<SocketAddr>().unwrap();
        let addr = Arc::new(addr);

        let mut threads = Vec::new();

        for i in 0..settings.threads {
            let settings_ref = settings.clone();
            let sender_ref = sender.clone();
            let addr_ref = addr.clone();

            threads.push(thread::spawn(move || {
                info!("Spawning thread {}", i);

                let mut l = Core::new().unwrap();
                let handle = l.handle();

                let tcp_listener = net2::TcpBuilder::new_v4().unwrap()
                    .reuse_port(true).unwrap()
                    .bind(addr_ref.clone().as_ref()).unwrap()
                    .listen(128).unwrap(); // limit for pending connections. https://stackoverflow.com/a/36597268/3392786

                let listener = TcpListener::from_listener(tcp_listener, addr_ref.as_ref(), &handle).unwrap();

                let server = listener.incoming().for_each(|(tcp, _)| {

                    let (writer, reader) = tcp.framed(LineCodec).split();
                    let service = (|| Ok(TcpListenerService::new(i, sender_ref.clone(), settings_ref.clone()))).new_service()?;

                    let responses = reader.and_then(move |req| service.call(req));
                    let server = writer.send_all(responses)
                        .then(|_| Ok(()));
                    handle.spawn(server);

                    Ok(())
                });
                l.run(server).unwrap();
            }));
        }

        info!("Listening at {} via TCP with {} threads...", addr, settings.threads);

        threads
    }
}

pub struct LineCodec;

impl Encoder for LineCodec {
    type Item = ();
    type Error = io::Error;

    fn encode(&mut self, msg: (), buf: &mut BytesMut) -> io::Result<()> {
        Ok(())
    }
}

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<String>> {
        if let Some(i) = buf.iter().position(|&b| b == b'\n') {
            let line = buf.split_to(i + 1);

            // Turn this data into a UTF string and return it in a Frame.
            match str::from_utf8(&line) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                             "invalid UTF-8")),
            }
        } else {
            Ok(None)
        }
    }
}

struct TcpListenerService {
    pub id: i32,
    pub name: String,
    pub tx_file_writer: SyncSender<FileWriterCommand>,
    settings: Arc<Settings>,
}

impl TcpListenerService {

    pub fn new(id: i32, tx_file_writer: SyncSender<FileWriterCommand>, settings: Arc<Settings>) -> Self {

        TcpListenerService {
            id,
            name: format!("server-tcp-{}", id),
            tx_file_writer,
            settings
        }
    }

}

impl Service for TcpListenerService {
    type Request = String;
    type Response = ();
    type Error = io::Error;
    type Future = FutureResult<Self::Response, Self::Error>;

    fn call(&self, req: Self::Request) -> Self::Future {
        debug!("Received a log line in {}", self.name);
        let sent_data = self.tx_file_writer
            .send(FileWriterCommand::Write(req.clone().into_bytes()));
        match sent_data {
            Ok(_)  => future::ok(()),
            Err(e) => future::err(io::Error::new(io::ErrorKind::Other,
                                     format!("Error trying to send a log line to write: {}", e)))
        }
    }
}
