//! Frame a MessagePack-RPC message (payload) from a transport with the total message length prefixed as
//! a 32-bit unsigned integer encoded in four (4) bytes.
//!
//! # Getting Started
//!
//! This can be used with the tokio-proto crate.
//!
//! ```
//! use framed_msgpack_rpc::{Codec, Message};
//! use tokio_io::{AsyncRead, AsyncWrite};
//! use tokio_io::codec::Framed;
//! use tokio_proto::pipeline::ServerProto;
//!
//! struct FramedMsgpackRpcProto;
//! 
//! impl<T: AsyncRead + AsyncWrite + 'static> ServerProto<T> for FramedMsgpackRpcProto {
//!    type Request = Message;
//!    type Response = Message;
//!    type Transport = Framed<T, Codec>
//!    type BindTransport = Result<Self::Transport, io::Error>;
//!
//!    fn bind_transport(&self, io: T) -> Self::BindTransport {
//!        Ok(io.framed(Codec::new()))
//!    }
//! }
//! ```

extern crate bytes;
extern crate env_logger;
extern crate framed_msgpack;
extern crate futures;
#[macro_use]
extern crate log;
extern crate rmpv;
extern crate tokio_core;
extern crate tokio_io;

pub use self::codec::Codec;

pub mod client;
mod codec;
pub mod message;
pub mod server;

