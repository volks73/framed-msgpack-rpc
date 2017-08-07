# framed-msgpack-rpc #

## What is framed-msgpack-rpc? ##

The framed-msgpack-rpc project is a [Rust](http://www.rust-lang.org) crate, a.k.a. library, that provides a codec for use with the [tokio-rs](https://tokio.rs/) project. [MessagePack-RPC](https://github.com/msgpack-rpc/msgpack-rpc)-based messages, or payloads, are sent and received with the total message length prefixed as a 32-bit unsigned integer encoded in four (4) [Big-Endian](https://en.wikipedia.org/wiki/Endianness) bytes. 

## Usage ##

First, add this to your `Cargo.toml`:

```toml
[dependencies]
framed-msgpack-rpc = { git = "https://github.com/volks73/framed-msgpack-rpc.git" }
```

Next, add this to your crate:

```rust
extern crate framed_msgpack_rpc;
```

## Getting Started ##

Clone this repository, then run the following command:

```
$ cargo run --example echo_server
```

This will run a server at `localhost:12345`. From another terminal, connect to the server and send framed-msgpack messages. The binary messages will be returned unchanged. A combination of the [netcat](https://en.wikipedia.org/wiki/Netcat), `nc`, application on UNIX-like systems, or [ncat](https://nmap.org/ncat/) for Windows, and the [panser](https://github.com/volks73/panser) application can be used to create framed-msgpack messages to send to the example echo server. For example, 

```
$ echo '[0,1,"sayHello",[]]' | panser --from json --to json --sized-output | nc localhost 12345 | panser --from msgpack --to json --sized-input --delimited-output 0Ah
[1,1,null,"Hello World!"]
$
```

## License ##

See the LICENSE file for more information about licensing and copyright.

