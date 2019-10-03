use crate::{
    protocol::{Request, RequestMsg, Response},
    PluginTrait,
};
use std::{
    fmt::{self, Display},
    io::{self, Write as _},
};

pub fn address(plugin_name: &str) -> String {
    format!(
        "ipc://{}/sockets/{}",
        env!("CARGO_MANIFEST_DIR"),
        plugin_name
    )
}

#[derive(Debug)]
pub enum ListenError {
    SocketFailed(nng::Error),
    ReceiveFailed(nng::Error),
    DeserializeFailed(bincode::Error),
    WriteFailed(io::Error),
    SerializeFailed(bincode::Error),
    SendFailed((nng::Message, nng::Error)),
}

impl Display for ListenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SocketFailed(err) => write!(f, "Failed to open socket: {}", err),
            Self::ReceiveFailed(err) => write!(f, "Failed to receive request: {}", err),
            Self::DeserializeFailed(err) => write!(f, "Failed to deserialize request: {}", err),
            Self::WriteFailed(err) => write!(f, "Failed to write message: {}", err),
            Self::SerializeFailed(err) => write!(f, "Failed to serialize response: {}", err),
            Self::SendFailed((msg, err)) => write!(f, "Failed to send response {:?}: {}", msg, err),
        }
    }
}

pub fn listen<P: PluginTrait>(plugin: &mut P) -> nng::Result<()> {
    let address = address(P::NAME);
    let server = nng::Socket::new(nng::Protocol::Rep0)?;
    server.listen(&address)?;
    log::info!("Listening at {}", address);
    loop {
        let mut msg = server.recv()?;
        let request = bincode::deserialize::<RequestMsg>(&msg).expect("handle me!");
        log::info!("<< Received {:#?}", request);
        let response = Response::new(request.respond(plugin));
        let serialized = bincode::serialize(&response).expect("handle me!");
        msg.clear();
        msg.write_all(&serialized).expect("handle me!");
        server.send(msg)?;
        log::info!(">> Sent {:#?}", response);
        if response.exit_requested() {
            server.close();
            return Ok(());
        }
    }
}

#[derive(Debug)]
pub enum SendError {
    DialFailed(nng::Error),
    SerializeFailed(bincode::Error),
    SendFailed((nng::Message, nng::Error)),
    ReceiveFailed(nng::Error),
    DeserializeFailed(bincode::Error),
}

impl Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DialFailed(err) => write!(f, "Failed to dial socket: {}", err),
            Self::SerializeFailed(err) => write!(f, "Failed to serialize request: {}", err),
            Self::SendFailed((msg, err)) => write!(f, "Failed to send request {:?}: {}", msg, err),
            Self::ReceiveFailed(err) => write!(f, "Failed to receive response: {}", err),
            Self::DeserializeFailed(err) => write!(f, "Failed to deserialize response: {}", err),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    socket: nng::Socket,
}

impl Client {
    pub fn new() -> nng::Result<Self> {
        let socket = nng::Socket::new(nng::Protocol::Req0)?;
        Ok(Self { socket })
    }

    pub fn send(&self, request: Request<'_>) -> Result<Response, SendError> {
        let address = address(request.plugin_name);
        self.socket.dial(&address).map_err(SendError::DialFailed)?;
        let serialized = bincode::serialize(&request.msg).map_err(SendError::SerializeFailed)?;
        self.socket
            .send(&serialized)
            .map_err(SendError::SendFailed)?;
        log::info!(">> Sent {:#?}", request);
        let msg = self.socket.recv().map_err(SendError::ReceiveFailed)?;
        let response =
            bincode::deserialize::<Response>(&msg).map_err(SendError::DeserializeFailed)?;
        log::info!("<< Received {:#?}", response);
        Ok(response)
    }
}
