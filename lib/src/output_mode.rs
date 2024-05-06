// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::cmp::Ordering;
use std::sync::Mutex;

use crate::Context;
use wayland_client::backend::ObjectId;
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

impl Dispatch<ZwlrOutputModeV1, Mutex<Option<ObjectId>>> for Context {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        data: &Mutex<Option<ObjectId>>,
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        let Some(head_id) = data.lock().unwrap().clone() else {
            return;
        };
        let Some(head) = state.output_heads.get_mut(&head_id) else {
            return;
        };
        let mode = head
            .modes
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
                if proxy.version() >= 3 {
                    proxy.release();
                }

                head.modes.shift_remove(&proxy.id());
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

impl PartialOrd for OutputMode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OutputMode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.width
            .cmp(&other.width)
            .then(self.height.cmp(&other.height))
            .then(self.refresh.cmp(&other.refresh))
            .reverse()
    }
}
