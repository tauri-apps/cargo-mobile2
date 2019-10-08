use crate::config;
use ginit_core::{
    cli::{Cli, CliInput},
    ipc::{self, Client},
    opts,
    protocol::{Features, Request, RequestMsg, ResponseMsg, Version, VERSION},
};
use std::{
    fmt::{self, Display},
    marker::PhantomData,
};

#[derive(Debug)]
pub enum Cause {
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

#[derive(Debug)]
pub struct Error {
    plugin_name: String,
    cause: Cause,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.cause {
            Cause::SendFailed(err) => write!(
                f,
                "Failed to send message to plugin {:?}: {}",
                self.plugin_name, err
            ),
            Cause::PluginFailed(err) => write!(
                f,
                "Plugin {:?} failed to handle request: {}",
                self.plugin_name, err
            ),
            Cause::ResponseMismatch { sent, received } => write!(
                f,
                "Sent a request of type {:?} to plugin {:?}, but received a response of type {:?}.",
                sent, self.plugin_name, received
            ),
            Cause::ProtocolMismatch {
                plugin_has,
                we_have,
            } => write!(
                f,
                "Plugin {:?} uses protocol v{}.{}, which is incompatible with the current v{}.{}.",
                self.plugin_name, plugin_has.0, plugin_has.1, we_have.0, we_have.1
            ),
        }
    }
}

fn send(client: &Client, request: Request<'_>) -> Result<ResponseMsg, Error> {
    let plugin_name = request.plugin_name;
    let request_ty = request.msg.ty();
    let response = client
        .send(request)
        .map_err(|cause| Error {
            plugin_name: plugin_name.to_owned(),
            cause: Cause::SendFailed(cause),
        })?
        .status
        .map_err(|cause| Error {
            plugin_name: plugin_name.to_owned(),
            cause: Cause::PluginFailed(cause),
        })?;
    let response_ty = response.ty();
    if response_ty == request_ty {
        Ok(response)
    } else {
        Err(Error {
            plugin_name: plugin_name.to_owned(),
            cause: Cause::ResponseMismatch {
                sent: request_ty,
                received: response_ty,
            },
        })
    }
}

#[derive(Debug)]
pub enum Unconfigured {}

#[derive(Debug)]
pub enum Configured {}

#[derive(Debug)]
pub struct Plugin<State> {
    client: Client,
    name: String,
    features: Features,
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

    fn transition<NewState, T, E>(
        self,
        result: Result<T, E>,
    ) -> Result<Plugin<NewState>, (E, Self)> {
        match result {
            Ok(_) => Ok(Plugin {
                client: self.client,
                name: self.name,
                features: self.features,
                description: self.description,
                _marker: PhantomData,
            }),
            Err(err) => Err((err, self)),
        }
    }

    pub fn shutdown(self) -> Result<(), Error> {
        if let ResponseMsg::Goodbye = send(&self.client, self.request(RequestMsg::Goodbye))? {
            Ok(())
        } else {
            unreachable!()
        }
    }

    pub fn cli(&self) -> Result<Option<Cli>, Error> {
        if let ResponseMsg::Cli { cli } = send(&self.client, self.request(RequestMsg::Cli))? {
            Ok(cli)
        } else {
            unreachable!()
        }
    }

    pub fn configure(self, config: &config::Umbrella) -> Result<Plugin<Configured>, (Error, Self)> {
        let result = send(
            &self.client,
            self.request(RequestMsg::Config {
                shared_config: config.shared().clone(),
                plugin_config: config.plugin(&self.name),
            }),
        );
        self.transition(result)
    }
}

impl Plugin<Unconfigured> {
    pub fn connect(client: Client, name: impl Into<String>) -> Result<Self, Error> {
        let name = name.into();
        if let ResponseMsg::Hello {
            protocol_version,
            features,
            description,
        } = send(&client, Request::new(&name, RequestMsg::Hello))?
        {
            if protocol_version.0 == VERSION.0 {
                Ok(Self {
                    client,
                    name,
                    features,
                    description,
                    _marker: PhantomData,
                })
            } else {
                Err(Error {
                    plugin_name: name.clone(),
                    cause: Cause::ProtocolMismatch {
                        plugin_has: protocol_version,
                        we_have: VERSION,
                    },
                })
            }
        } else {
            unreachable!()
        }
    }
}

impl Plugin<Configured> {
    pub fn init(&self, clobbering: opts::Clobbering) -> Result<(), Error> {
        if let ResponseMsg::Init =
            send(&self.client, self.request(RequestMsg::Init { clobbering }))?
        {
            Ok(())
        } else {
            unreachable!()
        }
    }

    pub fn exec(&self, input: CliInput, noise_level: opts::NoiseLevel) -> Result<(), Error> {
        if let ResponseMsg::Exec = send(
            &self.client,
            self.request(RequestMsg::Exec { input, noise_level }),
        )? {
            Ok(())
        } else {
            unreachable!()
        }
    }
}
