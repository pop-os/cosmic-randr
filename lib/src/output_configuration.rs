// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::{Context, Data, Message};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::Event;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;

impl Dispatch<ZwlrOutputConfigurationV1, Data> for Context {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        state.destroy_output_config();

        futures_lite::future::block_on(async {
            match event {
                Event::Succeeded => {
                    let _res = state.send(Message::ConfigurationSucceeded).await;
                }
                Event::Failed => {
                    let _res = state.send(Message::ConfigurationFailed).await;
                }
                Event::Cancelled => {
                    let _res = state.send(Message::ConfigurationCancelled).await;
                }
                _ => tracing::debug!(?event, "unknown event"),
            }
        });
    }
}
