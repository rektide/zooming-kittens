use crate::commands::longcat::{Backend, LongcatCommand};
#[cfg(feature = "niri")]
use futures::FutureExt;
use std::io;
use zk_desktop::Observer;
#[cfg(any(feature = "niri", feature = "umbriel"))]
use zk_desktop::SourceId;

// App-owned execution boundary: observation-only backends do not import niri.
#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "niri"), allow(dead_code))]
pub struct HeightRequest {
    pub target: zk_desktop::WindowKey,
    pub basis: zk_desktop::ObservationVersion,
    pub height: i32,
}

#[derive(Debug)]
#[cfg_attr(not(feature = "niri"), allow(dead_code))]
pub enum Transmission {
    Unsent(String),
    Rejected(String),
    Handled,
    PossiblySent(String),
}

pub enum Owner {
    #[cfg(feature = "niri")]
    Niri(zk_niri::Source),
    #[cfg(feature = "umbriel")]
    Umbriel(zk_umbriel::Source),
}

impl Owner {
    pub async fn shutdown(self) -> io::Result<()> {
        match self {
            #[cfg(feature = "niri")]
            Self::Niri(owner) => owner.shutdown().await.map_err(io::Error::other),
            #[cfg(feature = "umbriel")]
            Self::Umbriel(owner) => owner.shutdown().await.map_err(io::Error::other),
        }
    }
}

pub async fn subscribe(
    command: &LongcatCommand,
) -> io::Result<(Owner, Observer, Option<super::Executor>)> {
    match command.backend {
        #[cfg(feature = "niri")]
        Backend::Niri => {
            let id = SourceId::new(zk_desktop::Backend::Niri, "longcat");
            let (owner, observer) = if let Some(path) = &command.socket {
                zk_niri::subscribe(path, id, zk_niri::Options::default()).await?
            } else {
                zk_niri::subscribe_env(id, zk_niri::Options::default()).await?
            };
            let actuator = owner.actuator();
            let executor: super::Executor = std::sync::Arc::new(move |request| {
                let actuator = actuator.clone();
                async move {
                    match actuator
                        .set_height(zk_niri::HeightRequest {
                            target: request.target,
                            basis: request.basis,
                            height: request.height,
                        })
                        .await
                    {
                        zk_niri::Transmission::Unsent(message) => Transmission::Unsent(message),
                        zk_niri::Transmission::Rejected(message) => Transmission::Rejected(message),
                        zk_niri::Transmission::Handled => Transmission::Handled,
                        zk_niri::Transmission::PossiblySent(message) => {
                            Transmission::PossiblySent(message)
                        }
                    }
                }
                .boxed()
            });
            Ok((Owner::Niri(owner), observer, Some(executor)))
        }
        #[cfg(not(feature = "niri"))]
        Backend::Niri => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "feature `niri` is disabled in this build",
        )),
        #[cfg(feature = "umbriel")]
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
        #[cfg(not(feature = "umbriel"))]
        Backend::Umbriel => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "feature `umbriel` is disabled in this build",
        )),
    }
}
