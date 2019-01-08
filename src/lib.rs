
#![feature(try_trait)]
#![feature(custom_attribute)]

#[macro_use]
extern crate log;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate toml;
extern crate config;

extern crate net2;

extern crate chrono;
extern crate time;
extern crate glob;
extern crate regex;

extern crate futures;
extern crate tokio_core;
#[macro_use]
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;

extern crate bytes;

pub mod listener;
pub mod writer;
pub mod settings;

use std::net::SocketAddr;
use std::thread::{self, JoinHandle};
use std::sync::Arc;

use tokio_core::reactor::Core;

use settings::{Settings, ProtocolType};
use listener::Listener;
use writer::file_writer::FileWriter;

use futures::future::{self, FutureResult};
use futures::{Stream, Sink, Future};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::{Framed, Encoder, Decoder};

use bytes::BytesMut;
use tokio_proto::TcpServer;
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;
use tokio_service::NewService;

use std::sync::mpsc::SyncSender;

use std::io;
use std::str;
use std::borrow::Borrow;

pub struct App;

impl App {

    pub fn start_up(settings: Arc<Settings>) {

        let mut file_writer = FileWriter::new(settings.buffer_bound, settings.filewriter.clone());

        let conn_threads = Listener::start(settings.clone(), file_writer.tx.clone());

        file_writer.start();
        for t in conn_threads {
            t.join().unwrap();
        }

    }
}
