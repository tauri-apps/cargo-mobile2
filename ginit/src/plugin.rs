use crate::config;
use ginit_core::{
    cli::{Cli, CliInput},
    ipc::{self, Client},
    opts,
    protocol::{PluginType, Request, RequestMsg, ResponseMsg, Version, VERSION},
};
use std::{
    fmt::{self, Display},
    marker::PhantomData,
};

#[derive(Debug)]
pub enum Error {
    SendFailed(ipc::SendError),
    PluginFailed(String),
    ResponseMismatch {
        sent: &'static str,
        received: &'static str,
    },
    ProtocolMismatch {
        plugin_has: Version,
        we_have: Version,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendFailed(err) => write!(f, "Failed to send message to plugin: {}", err),
            Self::PluginFailed(err) => write!(f, "Plugin failed to handle request: {}", err),
            Self::ResponseMismatch { sent, received } => write!(
                f,
                "Sent a request of type {:?}, but received a response of type {:?}.",
                sent, received
            ),
            Self::ProtocolMismatch {
                plugin_has,
                we_have,
            } => write!(
                f,
                "Plugin uses protocol v{}.{}, which is incompatible with the current v{}.{}.",
                plugin_has.0, plugin_has.1, we_have.0, we_have.1
            ),
        }
    }
}

fn send(client: &Client, request: Request<'_>) -> Result<ResponseMsg, Error> {
    let request_ty = request.msg.ty();
    let response = client
        .send(request)
        .map_err(Error::SendFailed)?
        .status
        .map_err(Error::PluginFailed)?;
    let response_ty = response.ty();
    if response_ty == request_ty {
        Ok(response)
    } else {
        Err(Error::ResponseMismatch {
            sent: request_ty,
            received: response_ty,
        })
    }
}

#[derive(Debug)]
pub enum Unconfigured {}

#[derive(Debug)]
pub enum Configured {}

#[derive(Debug)]
pub struct Plugin<State> {
    name: String,
    plugin_type: PluginType,
    description: String,
    _marker: PhantomData<State>,
}

impl<State> Plugin<State> {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    fn request(&self, msg: RequestMsg) -> Request<'_> {
        Request::new(&self.name, msg)
    }

    fn transition<NewState>(self) -> Plugin<NewState> {
        Plugin {
            name: self.name,
            plugin_type: self.plugin_type,
            description: self.description,
            _marker: PhantomData,
        }
    }

    pub fn shutdown(self, client: &Client) -> Result<(), Error> {
        if let ResponseMsg::Goodbye = send(client, self.request(RequestMsg::Goodbye))? {
            Ok(())
        } else {
            unreachable!()
        }
    }

    pub fn cli(&self, client: &Client) -> Result<Option<Cli>, Error> {
        if let ResponseMsg::Cli { cli } = send(client, self.request(RequestMsg::Cli))? {
            Ok(cli)
        } else {
            unreachable!()
        }
    }

    pub fn configure(
        self,
        client: &Client,
        config: &config::Umbrella,
    ) -> Result<Plugin<Configured>, Error> {
        if let ResponseMsg::Config = send(
            client,
            self.request(RequestMsg::Config {
                shared_config: config.shared().clone(),
                plugin_config: config.plugin(&self.name),
            }),
        )? {
            Ok(self.transition())
        } else {
            unreachable!()
        }
    }
}

impl Plugin<Unconfigured> {
    pub fn connect(client: &Client, name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if let ResponseMsg::Hello {
            protocol_version,
            plugin_type,
            description,
        } = send(client, Request::new(&name, RequestMsg::Hello))?
        {
            if protocol_version.0 == VERSION.0 {
                Ok(Self {
                    name,
                    plugin_type,
                    description,
                    _marker: PhantomData,
                })
            } else {
                Err(Error::ProtocolMismatch {
                    plugin_has: protocol_version,
                    we_have: VERSION,
                })
            }
        } else {
            unreachable!()
        }
    }
}

impl Plugin<Configured> {
    pub fn init(&self, client: &Client) -> Result<(), Error> {
        if let ResponseMsg::Init = send(client, self.request(RequestMsg::Init))? {
            Ok(())
        } else {
            unreachable!()
        }
    }

    pub fn exec(
        &self,
        client: &Client,
        input: CliInput,
        noise_level: opts::NoiseLevel,
    ) -> Result<(), Error> {
        if let ResponseMsg::Exec = send(
            client,
            self.request(RequestMsg::Exec { input, noise_level }),
        )? {
            Ok(())
        } else {
            unreachable!()
        }
    }
}
