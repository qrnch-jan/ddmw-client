//! Authentication and unauthentication.

use std::borrow::Borrow;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use tokio::io::{AsyncRead, AsyncWrite};

use tokio_util::codec::Framed;

use serde::Deserialize;

use blather::Telegram;

use crate::utils;
use crate::Error;


/// Authentication context used to signal how to authenticate a connection.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Auth {
  /// Account name used to authenticate.
  pub name: Option<String>,

  /// Load raw account passphrase from the specified filename.
  #[serde(rename = "pass-file")]
  pub pass_file: Option<String>,

  /// Raw account passphrase to authenticate with.  Only used if `name` has
  /// been set.
  pub pass: Option<String>,

  /// Use the specified file for authentication token storage.
  #[serde(rename = "token-file")]
  pub token_file: Option<String>,

  /// Authentication token.
  pub token: Option<String>
}


impl Auth {
  /// Return `true` if there's either a raw passphrase set in `pass` or a
  /// passphrase file has beeen set in `pass_file`.  This function does not
  /// validate if the passphrase file exists or is accessible.
  pub fn have_pass(&self) -> bool {
    self.pass.is_some() || self.pass_file.is_some()
  }

  /// Get passphrase.
  ///
  /// Return the raw `pass` field if set.  Otherwise, load the `pass_file` if
  /// set, and return error if the passphrase could not be loaded from the
  /// file.
  ///
  /// If neither `pass` nor `pass_file` have been set, return an error.
  pub fn get_pass(&self) -> Result<String, Error> {
    // 1. Return raw passphrase if set
    // 2. Load raw passphrase from file, if set.  Return error if file could
    //    not be read.
    // 3. Return `Ok(None)`
    if let Some(pass) = &self.pass {
      Ok(pass.clone())
    } else if let Some(fname) = &self.pass_file {
      if let Some(pass) = utils::read_single_line(fname) {
        Ok(pass)
      } else {
        return Err(Error::invalid_cred(
          "Unable to read passphrase from file"
        ));
      }
    } else {
      Err(Error::invalid_cred("Missing passphrase"))
    }
  }

  /// Get authentication token.
  ///
  /// Return the raw `token` field if set.  Otherwise, check if `token_file` is
  /// set.  If it is, then attempt to load the token from it.  If the file
  /// _does not_ exist, then:
  /// - Return `Ok(None)` if account name and pass(file) have been set.
  /// - Return error if account name has not been set.
  pub fn get_token(&self) -> Result<Option<String>, Error> {
    // 1. Return raw token if set.
    // 2. If a token file has been specified, then:
    //    - If file exists, then read token from it.
    //    - If file does not exist, then:
    //      - If username is set and pass(file) is set, then return `Ok(None)`,
    //        assuming the caller wants to request a token and store it in the
    //        specified file.
    //      - If neither username nor pass(file) is set, then return an error,
    //        since the token file is missing.
    if let Some(tkn) = &self.token {
      Ok(Some(tkn.clone()))
    } else if let Some(fname) = &self.token_file {
      let fname = Path::new(&fname);
      if fname.exists() {
        // Token file exists, attempt to load token from it
        if let Some(tkn) = utils::read_single_line(fname) {
          // Got it, hopefully.  The server will validate it.
          Ok(Some(tkn))
        } else {
          // Unable to read file.
          Err(Error::invalid_cred("Unable to read token from file"))
        }
      } else if self.name.is_none() {
        // Missing account name, so clearly the authentication call won't be
        // able to request a token to be stored in the non-existent file.
        Err(Error::invalid_cred("Unable to read token from file"))
      } else if self.pass.is_none() && self.pass_file.is_none() {
        // Have account name, but no passphrase, so authentication can't
        // succeed.
        Err(Error::invalid_cred("Missing passphrase for token request"))
      } else {
        // Token file does not exist, but an account name and pass(file) was
        // specified, so assume the caller will request an authentication token
        // and store to the specified location.
        Ok(None)
      }
    } else {
      // Neither token nor token file was specified
      Ok(None)
    }
  }


  /// Helper function for authenticating a connection.
  ///
  /// Authenticates the connection specified in `conn`, using the credentials
  /// stored in the `Auth` buffer using the following logic:
  ///
  /// 1. If a raw token has been supplied in the `token` field, then attempt
  ///    to authenticate with it and return the results.
  /// 2. If a `token_file` has been been set, then:
  ///    - If the file exists, try to load the authentication token,
  ///      authenticate with it, and return the results.
  ///    - If the file does not exist, then:
  ///      - If account name and/or passphrase have not been set, then return
  ///        error.
  ///      - If account name and passphrase have been set, then continue.
  /// 3. Make sure that an account name and a passphrase has been set.
  ///    The passphrase is either set from the `pass` field or by loading the
  ///    contents of the file in `pass_file`.  Return error account name or
  ///    passphrase can not be acquired.
  /// 4. Authenticate using account name and passphrase.  If a `token_file` was
  ///    specified, then request an authentication token and store it in
  ///    `token_file` on success.  Return error on failure.
  pub async fn authenticate<C>(
    &self,
    conn: &mut Framed<C, blather::Codec>
  ) -> Result<Option<String>, Error>
  where
    C: AsyncRead + AsyncWrite + Unpin
  {
    // Attempt to get token.
    // This will return Ok(None) if there's no token to be added to the `Auth`
    // server request, but it can also mean that the caller wants a token to be
    // *requested* (the `token_file` field needs to be checked for this).
    let tkn = self.get_token()?;

    if let Some(tkn) = tkn {
      // Authenticate using the token
      token(conn, CredStore::Buf(tkn)).await?;

      // Token authentications do not yield new tokens
      return Ok(None);
    }

    // If a token authentication wasn't performed, then require an account.
    if let Some(accname) = &self.name {
      // Get required passphrase.  (Note that this will return error if a
      // password can't be retrieved).
      let pass = self.get_pass()?;

      // Authenticate using account name and passphrase.  If a token file has
      // been set, then at this point it is safe to assume the caller wants to
      // request a token.
      let opttkn = accpass(
        conn,
        accname,
        CredStore::Buf(pass),
        self.token_file.is_some()
      )
      .await?;

      // If a token was returned, and a token file was specified, then attempt
      // to write the token to the file.
      if let Some(tkn) = opttkn {
        // (token_file should always be Some() if an opttkn was returned)
        if let Some(fname) = &self.token_file {
          let mut f = File::create(fname)?;
          f.write(tkn.as_bytes())?;
        }
        return Ok(Some(tkn));
      }

      // Successfully authenticated using user name and passphrase
      return Ok(None);
    }

    // Token authetication failed and no account name/password was passed, so
    // error out.
    Err(Error::invalid_cred("Missing credentials"))
  }
}

/// Choose where an a token/passphrase is fetched from.
pub enum CredStore {
  /// Credential is stored in a string.
  Buf(String),

  /// Credential is stored in a file.
  File(PathBuf)
}


/// Attempt to authenticate using an authentication token.
///
/// The token can be stored in either a string buffer or file.
pub async fn token<T, O>(
  conn: &mut Framed<T, blather::Codec>,
  tkn: O
) -> Result<(), Error>
where
  O: Borrow<CredStore>,
  T: AsyncRead + AsyncWrite + Unpin
{
  let tkn = match tkn.borrow() {
    CredStore::Buf(s) => s.clone(),
    CredStore::File(p) => {
      if let Some(t) = utils::read_single_line(p) {
        t
      } else {
        return Err(Error::invalid_cred("Unable to read token from file"));
      }
    }
  };

  let mut tg = Telegram::new_topic("Auth")?;
  tg.add_param("Tkn", tkn)?;
  crate::sendrecv(conn, &tg).await?;
  Ok(())
}


/// Attempt to authenticate using an account name and a passphrase.
///
/// Optionally request an authentication token.
///
/// On success, return `Ok(None)` if authentication token was not requested.
/// Return `Ok(Some(String))` with the token string if it was requested.
pub async fn accpass<T, A, P>(
  conn: &mut Framed<T, blather::Codec>,
  accname: A,
  pass: P,
  reqtkn: bool
) -> Result<Option<String>, Error>
where
  A: AsRef<str>,
  P: Borrow<CredStore>,
  T: AsyncRead + AsyncWrite + Unpin
{
  let mut tg = Telegram::new_topic("Auth")?;
  tg.add_param("AccName", accname.as_ref())?;

  let pass = match pass.borrow() {
    CredStore::Buf(s) => s.clone(),
    CredStore::File(p) => {
      if let Some(pass) = utils::read_single_line(p) {
        pass
      } else {
        return Err(Error::invalid_cred(
          "Unable to read passphrase from file"
        ));
      }
    }
  };
  tg.add_param("Pass", pass)?;

  if reqtkn {
    tg.add_param("ReqTkn", "True")?;
  }
  let params = crate::sendrecv(conn, &tg).await?;

  if reqtkn {
    let s = params.get_str("Tkn");
    if let Some(s) = s {
      Ok(Some(s.to_string()))
    } else {
      Ok(None)
    }
  } else {
    Ok(None)
  }
}


/// Return ownership of a connection to the built-in _unauthenticated_ account.
pub async fn unauthenticate<T: AsyncRead + AsyncWrite + Unpin>(
  conn: &mut Framed<T, blather::Codec>
) -> Result<(), Error> {
  let tg = Telegram::new_topic("Unauth")?;

  crate::sendrecv(conn, &tg).await?;

  Ok(())
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
