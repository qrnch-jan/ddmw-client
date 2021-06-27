//! Message sending and receiving functions.

pub mod recv;
pub mod send;

pub use recv::{recv, recvloop, recvloop_a};
pub use send::send;

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
