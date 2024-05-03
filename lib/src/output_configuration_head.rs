// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::Context;

use cosmic_protocols::output_management::v1::client::zcosmic_output_configuration_head_v1::ZcosmicOutputConfigurationHeadV1;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for Context {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        _event: <ZwlrOutputConfigurationHeadV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZcosmicOutputConfigurationHeadV1, ()> for Context {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicOutputConfigurationHeadV1,
        _event: <ZcosmicOutputConfigurationHeadV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
    }
}
