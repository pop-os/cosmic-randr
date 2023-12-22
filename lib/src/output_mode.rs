// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::{Context, Data};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::Event as ZwlrOutputModeEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
    pub wlr_mode: ZwlrOutputModeV1,
}

impl Dispatch<ZwlrOutputModeV1, Data> for Context {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        let mode = state
            .output_modes
            .entry(proxy.id())
            .or_insert_with(|| OutputMode::new(proxy.clone()));

        match event {
            ZwlrOutputModeEvent::Size { width, height } => {
                (mode.width, mode.height) = (width, height);
            }

            ZwlrOutputModeEvent::Refresh { refresh } => {
                mode.refresh = refresh;
            }

            ZwlrOutputModeEvent::Preferred => {
                mode.preferred = true;
            }

            ZwlrOutputModeEvent::Finished => {
                proxy.release();

                if let Err(why) = state.remove_mode(&proxy.id()) {
                    tracing::error!(?why, id = ?proxy.id(), "failed to remove mode");
                }
            }

            _ => tracing::debug!(?event, "unknown event"),
        }
    }
}

impl OutputMode {
    #[must_use]
    pub fn new(wlr_mode: ZwlrOutputModeV1) -> Self {
        Self {
            width: 0,
            height: 0,
            refresh: 0,
            preferred: false,
            wlr_mode,
        }
    }
}
