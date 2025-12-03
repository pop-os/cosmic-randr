// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

mod channel;
pub use channel::{Receiver, Sender, channel};

pub mod context;
pub use context::Context;

pub mod output_configuration;
pub mod output_configuration_head;
pub mod output_head;
pub mod output_manager;

pub mod output_mode;
pub use output_mode::OutputMode;

pub use cosmic_protocols::output_management::v1::client::zcosmic_output_head_v1::{
    AdaptiveSyncAvailability, AdaptiveSyncStateExt,
};
pub mod wl_registry;

use tokio::io::Interest;
use wayland_client::backend::WaylandError;
use wayland_client::{Connection, DispatchError, EventQueue};

/// Creates a wayland client connection with state for handling wlr outputs.
///
/// # Errors
///
/// Returns error if there are any wayland client connection errors.
pub fn connect(sender: Sender) -> Result<(Context, EventQueue<Context>), Error> {
    Context::connect(sender)
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Debug)]
pub enum Message {
    ConfigurationCancelled,
    ConfigurationFailed,
    ConfigurationSucceeded,
    ManagerDone,
    ManagerFinished,
    Unsupported,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("cannot free resources of destroyed output mode")]
    ReleaseOutputMode,
    #[error("wayland client context error")]
    WaylandContext(#[from] wayland_client::backend::WaylandError),
    #[error("wayland client dispatch error")]
    WaylandDispatch(#[from] wayland_client::DispatchError),
    #[error("wayland connection error")]
    WaylandConnection(#[from] wayland_client::ConnectError),
    #[error("wayland object ID invalid")]
    WaylandInvalidId(#[from] wayland_client::backend::InvalidId),
}

pub async fn async_dispatch<Data>(
    connection: &Connection,
    event_queue: &mut EventQueue<Data>,
    data: &mut Data,
) -> Result<usize, DispatchError> {
    let dispatched = event_queue.dispatch_pending(data)?;

    if dispatched > 0 {
        return Ok(dispatched);
    }

    connection.flush()?;

    if let Some(guard) = connection.prepare_read() {
        {
            let fd = guard.connection_fd();
            let fd = tokio::io::unix::AsyncFd::new(fd).unwrap();

            loop {
                match fd.ready(Interest::ERROR | Interest::READABLE).await {
                    Ok(async_guard) => {
                        if async_guard.ready().is_readable() {
                            break;
                        }
                    }

                    Err(why) if why.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(why) => return Err(DispatchError::Backend(WaylandError::Io(why))),
                }
            }
        }

        if let Err(why) = guard.read() {
            if let WaylandError::Io(ref error) = why {
                if error.kind() != std::io::ErrorKind::WouldBlock {
                    return Err(DispatchError::Backend(why));
                }
            } else {
                return Err(DispatchError::Backend(why));
            }
        }
    }

    event_queue.dispatch_pending(data)
}
