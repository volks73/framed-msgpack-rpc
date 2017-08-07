// Portions of this were taken from the [rmp-rpc](https://github.com/little-dude/rmp-rpc) project.

use std::io;
use rmpv::{Integer, Utf8String, Value};
use std::convert::From;

/// Represents a `MessagePack-RPC` message as described in the
/// [specifications](https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md#messagepack-rpc-protocol-specification)
#[derive(PartialEq, Clone, Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

/// Represents a `MessagePack-RPC` request as described in the
/// [specifications](https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md#messagepack-rpc-protocol-specification)
#[derive(PartialEq, Clone, Debug)]
pub struct Request {
    pub id: u32,
    pub method: String,
    pub params: Vec<Value>,
}

/// Represents a `MessagePack-RPC` response as described in the
/// [specifications](https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md#messagepack-rpc-protocol-specification)
#[derive(PartialEq, Clone, Debug)]
pub struct Response {
    pub id: u32,
    pub result: Result<Value, Value>,
}

/// Represents a `MessagePack-RPC` notification as described in the
/// [specifications](https://github.com/msgpack-rpc/msgpack-rpc/blob/master/spec.md#messagepack-rpc-protocol-specification)
#[derive(PartialEq, Clone, Debug)]
pub struct Notification {
    pub method: String,
    pub params: Vec<Value>,
}

const REQUEST_MESSAGE: u64 = 0;
const RESPONSE_MESSAGE: u64 = 1;
const NOTIFICATION_MESSAGE: u64 = 2;

impl Message {
    /// Converts a MessagePack value to a MessagePack-RPC message.
    ///
    /// This conversion can fail if the MessagePack value does not match the msgpack-rpc
    /// specification.
    pub fn from_value(v: Value) -> io::Result<Message> {
        if let Value::Array(ref array) = v {
            if array.len() < 3 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "The message does not have at least three array elements"))
            }
            if let Value::Integer(msg_type) = array[0] {
                match msg_type.as_u64() {
                    Some(REQUEST_MESSAGE) => {
                        return Ok(Message::Request(Request::from_value(array)?));
                    }
                    Some(RESPONSE_MESSAGE) => {
                        return Ok(Message::Response(Response::from_value(array)?));
                    }
                    Some(NOTIFICATION_MESSAGE) => {
                        return Ok(Message::Notification(Notification::from_value(array)?));
                    }
                    _ => {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown message type"))
                    }
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Message type is not an integer")) 
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The message must be an array type according to the msgpack-rpc specification"))
        }
    }

    /// Consumes this `Message` and converts it to a MessagePack value.
    pub fn to_value(self) -> Value {
        match self {
            Message::Request(r) => r.to_value(),
            Message::Response(r) => r.to_value(),
            Message::Notification(n) => n.to_value(),
        }
    }
}

impl Notification {
    fn from_value(array: &[Value]) -> io::Result<Self> {
        if array.len() != 3 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The notification does not have three array elements"))
        }
        let method = if let Value::String(ref method) = array[1] {
            method
                .as_str()
                .and_then(|s| Some(s.to_string()))
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "The notification method is not a string"))?
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The notification does not have a method"));
        };
        let params = if let Value::Array(ref params) = array[2] {
            params.clone()
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The notification does not have any parameters"));
        };
        Ok(Notification {
            method: method,
            params: params,
        })
    }

    fn to_value(self) -> Value {
        Value::Array(
            vec![
                Value::Integer(Integer::from(NOTIFICATION_MESSAGE)),
                Value::String(Utf8String::from(self.method.as_str())),
                Value::Array(self.params.to_owned()),
            ]
        )
    }
}

impl Request {
    fn from_value(array: &[Value]) -> io::Result<Self> {
        if array.len() != 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The request does not have four array elements"));
        }
        let id = if let Value::Integer(id) = array[1] {
            id.as_u64()
                .and_then(|id| Some(id as u32))
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "The request ID is not an integer"))?
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The request does not have an ID"));
        };
        let method = if let Value::String(ref method) = array[2] {
            method
                .as_str()
                .and_then(|s| Some(s.to_string()))
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "The request method is not a string"))?
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The request does not have a method"));
        };
        let params = if let Value::Array(ref params) = array[3] {
            params.clone()
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The request does not have any parameters"));
        };
        Ok(Request {
            id: id,
            method: method,
            params: params,
        })
    }

    fn to_value(self) -> Value {
        Value::Array(vec![
            Value::Integer(Integer::from(REQUEST_MESSAGE)),
            Value::Integer(Integer::from(self.id)),
            Value::String(Utf8String::from(self.method.as_str())),
            Value::Array(self.params),
        ])
    }
}

impl Response {
    fn from_value(array: &[Value]) -> io::Result<Self> {
        if array.len() != 4 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The response does not have four array elements"));
        }
        let id = if let Value::Integer(id) = array[1] {
            id.as_u64()
                .and_then(|id| Some(id as u32))
                .ok_or(io::Error::new(io::ErrorKind::InvalidData, "The response ID is not an integer"))?
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "The response does not have an ID"));
        };
        match array[2] {
            Value::Nil => Ok(Response {
                id: id,
                result: Ok(array[3].clone()),
            }),
            ref error => Ok(Response {
                id: id,
                result: Err(error.clone()),
            }),
        }
    }

    fn to_value(self) -> Value {
        let (error, result) = match self.result {
            Ok(ref result) => (Value::Nil, result.to_owned()),
            Err(ref err) => (err.to_owned(), Value::Nil),
        };
        Value::Array(vec![
            Value::Integer(Integer::from(RESPONSE_MESSAGE)),
            Value::Integer(Integer::from(self.id)),
            error,
            result,
        ])
    }
}

