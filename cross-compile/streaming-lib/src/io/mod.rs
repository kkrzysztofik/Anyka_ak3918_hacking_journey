extern crate byteorder;
extern crate bytes;

pub mod bits_errors;
pub mod bits_reader;
pub mod bits_writer;
pub mod bytes_errors;
pub mod bytes_reader;
pub mod bytes_writer;
pub mod bytesio_errors;

// Include bytesio module from bytesio.rs (avoiding module inception)
#[path = "bytesio.rs"]
mod bytesio_impl;
pub use bytesio_impl::{
    BatchWriteStats, NetType, TNetIO, TcpIO, TcpReadIO, TcpWriteIO, UdpIO, new_udpio_pair,
};
