//! Load and parse DDMW application configuration file.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::auth::Auth;

use figment::{
  providers::{Format, Toml},
  Figment
};

use crate::conn::ProtAddr;
use crate::err::Error;
use crate::types::AppChannel;


#[derive(Debug, Default, Deserialize)]
pub struct Config {
  pub channel: Option<String>,
  pub auth: Option<Auth>,
  pub sender: Option<Sender>,
  pub receiver: Option<Receiver>
}

impl Config {
  pub fn set_appch(&mut self, appch: AppChannel) -> &mut Self {
    self.channel = Some(appch.to_string());
    self
  }

  pub fn get_appch(&self) -> Result<Option<AppChannel>, Error> {
    if let Some(appch) = &self.channel {
      match appch.parse::<AppChannel>() {
        Ok(appch) => {
          return Ok(Some(appch));
        }
        Err(e) => {
          let err = format!("AppChannel, {}", e);
          return Err(Error::parse(err));
        }
      }
    }
    Ok(None)
  }

  pub fn set_sender_msgif(&mut self, pa: ProtAddr) -> &mut Self {
    if self.sender.is_none() {
      self.sender = Some(Sender::default());
    }
    if let Some(ref mut sender) = self.sender {
      sender.msgif = Some(pa.to_string());
    }
    self
  }

  pub fn get_sender_msgif(&self) -> Result<Option<ProtAddr>, Error> {
    if let Some(sender) = &self.sender {
      if let Some(addr) = &sender.msgif {
        match addr.parse::<ProtAddr>() {
          Ok(addr) => {
            return Ok(Some(addr));
          }
          Err(e) => {
            let err = format!("ProtAddr, {}", e);
            return Err(Error::parse(err));
          }
        }
      }
    }
    Ok(None)
  }

  pub fn set_auth_account(&mut self, name: &str) -> &mut Self {
    if self.auth.is_none() {
      self.auth = Some(Auth::default());
    }
    if let Some(ref mut auth) = self.auth {
      auth.name = Some(name.to_string());
    }
    self
  }
  pub fn set_auth_pass(&mut self, pass: &str) -> &mut Self {
    if self.auth.is_none() {
      self.auth = Some(Auth::default());
    }
    if let Some(ref mut auth) = self.auth {
      auth.pass = Some(pass.to_string());
    }
    self
  }
  pub fn set_auth_pass_file(&mut self, passfile: &str) -> &mut Self {
    if self.auth.is_none() {
      self.auth = Some(Auth::default());
    }
    if let Some(ref mut auth) = self.auth {
      auth.pass_file = Some(passfile.to_string());
    }
    self
  }
  pub fn set_auth_token(&mut self, tkn: &str) -> &mut Self {
    if self.auth.is_none() {
      self.auth = Some(Auth::default());
    }
    if let Some(ref mut auth) = self.auth {
      auth.token = Some(tkn.to_string());
    }
    self
  }
  pub fn set_auth_token_file(&mut self, tknfile: &str) -> &mut Self {
    if self.auth.is_none() {
      self.auth = Some(Auth::default());
    }
    if let Some(ref mut auth) = self.auth {
      auth.token_file = Some(tknfile.to_string());
    }
    self
  }
}

#[derive(Debug, Default, Deserialize)]
pub struct Sender {
  pub mgmtif: Option<String>,
  pub msgif: Option<String>
}


#[derive(Debug, Default, Deserialize)]
pub struct Receiver {
  pub mgmtif: Option<String>,
  pub subif: Option<String>,
  #[serde(rename = "sub-retries")]
  pub sub_retries: Option<u32>,
  #[serde(rename = "sub-retry-delay")]
  pub sub_retry_delay: Option<String>,
  #[serde(rename = "push-listenif")]
  pub push_listenif: Option<String>
}


/// Load a DDMW application configuration file.
///
/// Attempt to load a configuration file in the following order:
/// 1. If `fname` has `Some` value, its value will be used.  Otherwise:
/// 2. If the environment variable `DDMW_APPCONF` is set, its value will be
///    used.  Otherwise:
/// 3. The filename `ddmwapp.toml`, in the current working directory, will be
///    used.
///
/// If none of these could be be found, `Ok(None)` will be returned.
///
/// # Example
/// Attempt to load a "hello.toml", and return a default `Config` buffer if
/// unsuccessful.
///
/// ```no_run
/// use std::path::Path;
/// use ddmw_client::conf::{Config, load};
/// use ddmw_client::Error;
/// fn get_conf() -> Result<Config, Error> {
///   let fname = Path::new("hello.toml");
///   Ok(load(Some(&fname))?.unwrap_or_default())
/// }
/// ```
pub fn load(fname: Option<&Path>) -> Result<Option<Config>, Error> {
  let f = match fname {
    Some(p) => p.to_path_buf(),
    None => match std::env::var_os("DDMW_APPCONF") {
      Some(val) => PathBuf::from(val),
      None => PathBuf::from("ddmwapp.toml")
    }
  };

  if !f.exists() {
    Ok(None)
  } else {
    let conf = Figment::new().merge(Toml::file(f)).extract()?;
    Ok(conf)
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
