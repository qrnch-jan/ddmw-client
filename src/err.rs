//! Error values.

use std::fmt;

use tokio::io;

use blather::Params;

/// `ddmw-client` error values.
#[derive(Debug)]
pub enum Error {
  /// An error occurred in the Blather communications library.
  Blather(String),

  /// A `std::io` or `tokio::io` error occurred.
  IO(String),

  /// A DDMW core server return `Fail`.  The `Params` buffer contains details
  /// about the error.
  ServerError(Params),

  /// A state was entered which was unexpected.  This can mean that the client
  /// expected to receive something from the server, but received something
  /// else, which may technically have been okay under different
  /// circumstances.
  BadState(String),

  /// A server disconnected or the client is in a disconnected state.
  Disconnected,

  /// A function or method was called with an invalid/unknown input.
  BadInput(String),

  /// A function or method was called with incomplete or ambiguous parameters.
  BadParams(String),

  /// Authentication was requested, but the authentication context is invalid
  /// (missing or invalid data).
  InvalidCredentials(String),

  /// Some expected data is missing.
  MissingData(String),

  Parse(String),

  Figment(String)
}

impl Error {
  pub fn invalid_cred(e: &str) -> Self {
    Self::InvalidCredentials(e.to_string())
  }
  pub fn miss_data<S: ToString>(e: S) -> Self {
    Self::MissingData(e.to_string())
  }
  pub fn bad_state<S: ToString>(e: S) -> Self {
    Self::BadState(e.to_string())
  }
  pub fn parse<S: ToString>(e: S) -> Self {
    Self::Parse(e.to_string())
  }
}


impl std::error::Error for Error {}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match &*self {
      Error::Blather(s) => write!(f, "Msg buffer error; {}", s),
      Error::IO(s) => write!(f, "I/O error; {}", s),
      Error::ServerError(p) => write!(f, "Server replied: {}", p),
      Error::BadState(s) => {
        write!(f, "Encountred an unexpected/bad state: {}", s)
      }
      Error::Disconnected => write!(f, "Disconnected"),
      Error::BadInput(s) => write!(f, "Bad input; {}", s),
      Error::BadParams(s) => write!(f, "Bad parameters; {}", s),
      Error::InvalidCredentials(s) => write!(f, "Invalid credentials; {}", s),
      Error::MissingData(s) => write!(f, "Missing data; {}", s),
      Error::Parse(s) => write!(f, "Parsing failed; {}", s),
      Error::Figment(s) => write!(f, "Figment error; {}", s)
    }
  }
}

impl From<blather::Error> for Error {
  fn from(err: blather::Error) -> Self {
    Error::Blather(err.to_string())
  }
}

impl From<io::Error> for Error {
  fn from(err: io::Error) -> Self {
    Error::IO(err.to_string())
  }
}

impl From<figment::Error> for Error {
  fn from(err: figment::Error) -> Self {
    Error::Figment(err.to_string())
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
