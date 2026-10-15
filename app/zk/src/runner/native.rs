use crate::commands::longcat::{Backend, LongcatCommand};
use std::io;
use zk_desktop::{Observer, SourceId};

pub enum Owner {
    Niri(zk_niri::Source),
    Umbriel(zk_umbriel::Source),
}

impl Owner {
    pub async fn shutdown(self) -> io::Result<()> {
        let result = match self {
            Self::Niri(owner) => owner.shutdown().await,
            Self::Umbriel(owner) => owner.shutdown().await,
        };
        result.map_err(io::Error::other)
    }
}

pub async fn subscribe(
    command: &LongcatCommand,
) -> io::Result<(Owner, Observer, Option<zk_niri::HeightActuator>)> {
    match command.backend {
        Backend::Niri => {
            let id = SourceId::new(zk_desktop::Backend::Niri, "longcat");
            let (owner, observer) = if let Some(path) = &command.socket {
                zk_niri::subscribe(path, id, zk_niri::Options::default()).await?
            } else {
                zk_niri::subscribe_env(id, zk_niri::Options::default()).await?
            };
            let actuator = owner.actuator();
            Ok((Owner::Niri(owner), observer, Some(actuator)))
        }
        Backend::Umbriel => {
            let endpoint = match &command.socket {
                Some(path) => zk_umbriel::Endpoint::new(path.clone()),
                None => zk_umbriel::Discovery::from_env()
                    .resolve()
                    .map_err(io::Error::other)?,
            };
            let (owner, observer) = zk_umbriel::subscribe(
                endpoint,
                SourceId::new(zk_desktop::Backend::Umbriel, "longcat"),
                zk_umbriel::Options::default(),
            )
            .await
            .map_err(io::Error::other)?;
            Ok((Owner::Umbriel(owner), observer, None))
        }
    }
}
