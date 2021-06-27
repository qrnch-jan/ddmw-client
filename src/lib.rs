//! Client library for creating integrations against DDMW.
//!
//! # Establishing connections & basic communication
//! Connections can be established to the DDMW server's client interfaces using
//! the [`connect`](conn::connect) function in the [`conn`] module.  This
//! module also provides functions for:
//! - sending commands and receiving replies.
//! - ask the server who owns the connection.
//!
//! The `connect` method supports optionally authenticating the connection, but
//! this can also be performed explicitly after the connection has been
//! established using the [`authenticate()`](auth::Auth::authenticate) method
//! in the [`auth`] module.
//!
//! # Application configuration
//! Most, if not all, DDMW applications will require a few common configuration
//! parameters.  To this end a common configuration format is specified in the
//! [`conf`] module.  There's a helper function for loading and parsing such a
//! configuration file.
//!
//! The configuration file is entirely optional, but it provides a common
//! configuration file structure for applications to use.
//!
//! # Probing the server
//! The DDMW servers' client interfaces support a few common commands which
//! are typically used to simply for low-level availability checks and for
//! quering the servers for static information.  The [`probe`] module contains
//! helper functions for accessing this type of data.
//!
//! # Data transfers
//! The primary role of integrations such as native DDMW applications or
//! proxies is to send and receiver messages or streams.  The [`msg`] and
//! [`strm`] modules provide functions for sending and receiving messages and
//! streams.
//!
//! # Management
//! To create management clients the [`mgmt`] module wrapper contains helper
//! functions for management commands.

//#![deny(missing_docs)]
//#![deny(missing_crate_level_docs)]
//#![deny(missing_doc_code_examples)]

pub mod auth;
pub mod conf;
pub mod conn;
pub mod err;
pub mod mgmt;
pub mod msg;
pub mod probe;
pub mod strm;
pub mod types;

mod utils;

pub use err::Error;

pub use conn::{expect_okfail, sendrecv};

pub use conf::Config;

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
