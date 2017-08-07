//! A simple TCP echo server using framed-msgpack-rpc codec.
//!
//! MessagePack-RPC messages, i.e. Request, Response, and Notifications, are received from a client
//! and transmitted back without modification (echo).

extern crate framed_msgpack_rpc;
extern crate futures;
extern crate rmpv;
extern crate tokio_io;
extern crate tokio_proto;
extern crate tokio_service;

use framed_msgpack_rpc::Codec;
use framed_msgpack_rpc::message::{Message, Response};
use futures::{future, Future, BoxFuture};
use rmpv::{Utf8String, Value};
use std::io;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_io::codec::Framed;
use tokio_proto::pipeline::ServerProto;
use tokio_proto::TcpServer;
use tokio_service::Service;

pub struct FramedMsgpackRpcProto;

impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for FramedMsgpackRpcProto {
    type Request = Message;
    type Response = Message;
    type Transport = Framed<T, Codec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(Codec::new()))
    }
}

pub struct Echo;

impl Service for Echo {
    type Request = Message;
    type Response = Message;
    type Error = io::Error;
    type Future = BoxFuture<Self::Response, Self::Error>;

    fn call(&self, msg: Self::Request) -> Self::Future {
        future::ok(msg).boxed()
    }
}

fn main() {
    let addr = "0.0.0.0:12345".parse().unwrap();
    let server = TcpServer::new(FramedMsgpackRpcProto, addr);
    server.serve(|| Ok(Echo));
}

