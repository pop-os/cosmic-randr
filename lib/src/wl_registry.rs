// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::{Context, Message};
use cosmic_protocols::output_management::v1::client::zcosmic_output_manager_v1::ZcosmicOutputManagerV1;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

impl Dispatch<wl_registry::WlRegistry, ()> for Context {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _data: &(),
        _conn: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            if "zwlr_output_manager_v1" == &interface[..] {
                if version < 2 {
                    tracing::error!(
                        "wlr-output-management protocol version {version} < 2 is not supported"
                    );

                    futures_lite::future::block_on(async {
                        let _ = state.send(Message::Unsupported).await;
                    });

                    return;
                }

                state.output_manager_version = version;
                state.output_manager = Some(registry.bind::<ZwlrOutputManagerV1, _, _>(
                    name,
                    version.min(4),
                    handle,
                    (),
                ));
            }
            if "zcosmic_output_manager_v1" == &interface[..] {
                state.cosmic_output_manager =
                    Some(registry.bind::<ZcosmicOutputManagerV1, _, _>(name, 1, handle, ()))
            }
        }
    }
}
