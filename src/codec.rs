use bytes::BytesMut;
use framed_msgpack::Codec as FramedMsgpack;
use message::Message;
use std::io;
use tokio_io::codec::{Decoder, Encoder};

#[derive(Debug, Clone, Copy)]
pub struct Codec {
    inner: FramedMsgpack,
}

impl Codec {
    pub fn new() -> Self {
        Codec {
            inner: FramedMsgpack::new()
        }
    }
}

impl Decoder for Codec {
    type Item = Message;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> io::Result<Option<Self::Item>> {
        match self.inner.decode(src)? {
            Some(v) => Ok(Some(Message::from_value(v)?)),
            None => Ok(None)
        }
    }
}

impl Encoder for Codec {
    type Item = Message;
    type Error = io::Error;

    fn encode(&mut self, msg: Self::Item, buf: &mut BytesMut) -> io::Result<()> {
        self.inner.encode(msg.to_value(), buf)
    }
}

impl Default for Codec {
    fn default() -> Self {
        Codec {
            inner: FramedMsgpack::new(),
        }
    }
}

