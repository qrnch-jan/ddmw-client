use ddmw_client::{
  conf, conn,
  msg::{
    self,
    send::{MsgInfo, Transport}
  }
};


#[tokio::main]
async fn main() {
  // Attempt to load a ddmwapp.toml configuration file.  Generate a default
  // configuration if loading was unsuccessful.
  let config = conf::load(None)
    .expect("Unable to load configuration")
    .unwrap_or_default();

  // Attempt to extract the application channel from the configuration.
  let appch = config
    .get_appch()
    .expect("Unable to parse application channel")
    .expect("Missing required application channel");

  // Attempt to extract the sender's msgif socket address from the
  // configuration.
  let protaddr = config
    .get_sender_msgif()
    .expect("Unable to parse ProtAddr")
    .expect("Missing required msgif");


  // Attempt to connect to sender node's message interface.  Optionally
  // authenticate the connection.
  let mut conn = conn::connect(protaddr, config.auth.as_ref())
    .await
    .expect("Unable to connect");

  // Prepare a context for the transmission layer
  let xfer = Transport { ch: appch };

  // Prepare a message context
  let mi = MsgInfo {
    cmd: 17,
    meta: None,
    payload: None
  };

  msg::send(&mut conn, &xfer, &mi)
    .await
    .expect("Unable to send message");
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
