//! Types that are used to describe server's data diode link properties.

use std::fmt;
use std::str::FromStr;

use crate::Error;

#[derive(Debug, PartialEq)]
pub enum Protocol {
  Ethernet,
  UDP
}

impl fmt::Display for Protocol {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let s = match self {
      Protocol::Ethernet => "ethernet",
      Protocol::UDP => "udp"
    };
    write!(f, "{}", s)
  }
}

impl FromStr for Protocol {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "ethernet" => Ok(Protocol::Ethernet),
      "udp" => Ok(Protocol::UDP),
      _ => Err(Error::BadInput(format!("Unknown ddlnk::Protocol '{}'", s)))
    }
  }
}


#[derive(Debug, PartialEq)]
pub enum ProtImpl {
  Pcap,
  Generic
}

impl fmt::Display for ProtImpl {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let s = match self {
      ProtImpl::Pcap => "pcap",
      ProtImpl::Generic => "generic"
    };
    write!(f, "{}", s)
  }
}

impl FromStr for ProtImpl {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    match s {
      "pcap" => Ok(ProtImpl::Pcap),
      "generic" => Ok(ProtImpl::Generic),
      _ => Err(Error::BadInput(format!("Unknown ddlnk::ProtImpl '{}'", s)))
    }
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn protocol_to_string() {
    let t = Protocol::Ethernet;
    let s = format!("{}", t);
    assert_eq!(s, "ethernet");

    let t = Protocol::UDP;
    let s = format!("{}", t);
    assert_eq!(s, "udp");
  }

  #[test]
  fn protimpl_to_string() {
    let t = ProtImpl::Pcap;
    let s = format!("{}", t);
    assert_eq!(s, "pcap");

    let t = ProtImpl::Generic;
    let s = format!("{}", t);
    assert_eq!(s, "generic");
  }

  #[test]
  fn string_to_protocol() {
    let t = "ethernet".parse::<Protocol>().unwrap();
    assert_eq!(t, Protocol::Ethernet);

    let t = "udp".parse::<Protocol>().unwrap();
    assert_eq!(t, Protocol::UDP);
  }

  #[test]
  fn string_to_protimpl() {
    let t = "pcap".parse::<ProtImpl>().unwrap();
    assert_eq!(t, ProtImpl::Pcap);

    let t = "generic".parse::<ProtImpl>().unwrap();
    assert_eq!(t, ProtImpl::Generic);
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
