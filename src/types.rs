//! Various types used when communicating with core servers.

pub mod node;

use std::fmt;
use std::str::FromStr;

use crate::err::Error;


/// Reference an account; with the option to implicitly reference self.
pub enum OptObjRef {
  Current,
  Id(i64),
  Name(String)
}

/// Reference an account, either by numeric identifier or name.
pub enum ObjRef {
  Id(i64),
  Name(String)
}

impl FromStr for ObjRef {
  type Err = Error;

  /// Parse a `&str` and turn it into an `ObjRef`.
  fn from_str(o: &str) -> Result<Self, Self::Err> {
    match o.parse::<i64>() {
      Ok(id) => Ok(ObjRef::Id(id)),
      Err(_) => Ok(ObjRef::Name(o.to_string()))
    }
  }
}


pub enum AppChannel {
  Num(u8),
  Name(String)
}

impl FromStr for AppChannel {
  type Err = Error;

  /// Parse a `&str` and turn it into an `AppChannel`.
  fn from_str(ch: &str) -> Result<Self, Self::Err> {
    match ch.parse::<u8>() {
      Ok(ch) => Ok(AppChannel::Num(ch)),
      Err(_) => Ok(AppChannel::Name(ch.to_string()))
    }
  }
}

impl fmt::Display for AppChannel {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AppChannel::Num(ch) => {
        write!(f, "{}", ch)
      }
      AppChannel::Name(ch) => {
        write!(f, "{}", ch)
      }
    }
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
