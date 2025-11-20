// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::sync::Mutex;

use crate::{Context, OutputMode};

use cosmic_protocols::output_management::v1::client::zcosmic_output_head_v1::AdaptiveSyncAvailability;
use cosmic_protocols::output_management::v1::client::zcosmic_output_head_v1::AdaptiveSyncStateExt;
use cosmic_protocols::output_management::v1::client::zcosmic_output_head_v1::Event as ZcosmicOutputHeadEvent;
use cosmic_protocols::output_management::v1::client::zcosmic_output_head_v1::ZcosmicOutputHeadV1;
use indexmap::IndexMap;
use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::EVT_MODE_OPCODE;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Clone, Debug, PartialEq)]
pub struct OutputHead {
    pub adaptive_sync: Option<AdaptiveSyncStateExt>,
    pub adaptive_sync_support: Option<AdaptiveSyncAvailability>,
    pub current_mode: Option<ObjectId>,
    pub description: String,
    pub enabled: bool,
    pub make: String,
    pub model: String,
    pub modes: IndexMap<ObjectId, OutputMode>,
    pub name: String,
    pub physical_height: i32,
    pub physical_width: i32,
    pub position_x: i32,
    pub position_y: i32,
    pub scale: f64,
    pub serial_number: String,
    pub transform: Option<Transform>,
    pub mirroring: Option<String>,
    pub xwayland_primary: Option<bool>,
    pub wlr_head: ZwlrOutputHeadV1,
    pub cosmic_head: Option<ZcosmicOutputHeadV1>,
}

impl Dispatch<ZwlrOutputHeadV1, ()> for Context {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        let head = state
            .output_heads
            .get_mut(&proxy.id())
            .expect("Inert WlrOutputHead");

        match event {
            ZwlrOutputHeadEvent::Name { name } => {
                head.name = name;
            }

            ZwlrOutputHeadEvent::Description { description } => {
                head.description = description;
            }

            ZwlrOutputHeadEvent::PhysicalSize { width, height } => {
                (head.physical_width, head.physical_height) = (width, height);
            }

            ZwlrOutputHeadEvent::Mode { mode } => {
                *mode
                    .data::<Mutex<Option<ObjectId>>>()
                    .unwrap()
                    .lock()
                    .unwrap() = Some(proxy.id());
                head.modes.insert(mode.id(), OutputMode::new(mode));
            }

            ZwlrOutputHeadEvent::Enabled { enabled } => {
                head.enabled = !matches!(enabled, 0);
            }

            ZwlrOutputHeadEvent::CurrentMode { mode } => {
                head.current_mode = Some(mode.id());
            }

            ZwlrOutputHeadEvent::Position { x, y } => {
                (head.position_x, head.position_y) = (x, y);
            }

            ZwlrOutputHeadEvent::Transform { transform } => {
                head.transform = transform.into_result().ok();
            }

            ZwlrOutputHeadEvent::Scale { scale } => {
                head.scale = scale;
            }

            ZwlrOutputHeadEvent::Finished => {
                if proxy.version() >= 3 {
                    proxy.release();
                }
                state.output_heads.remove(&proxy.id());
            }

            ZwlrOutputHeadEvent::Make { make } => {
                head.make = make;
            }

            ZwlrOutputHeadEvent::Model { model } => {
                head.model = model;
            }

            ZwlrOutputHeadEvent::SerialNumber { serial_number } => {
                head.serial_number = serial_number;
            }

            ZwlrOutputHeadEvent::AdaptiveSync { state } => {
                head.adaptive_sync = match state.into_result().ok() {
                    Some(AdaptiveSyncState::Enabled) => Some(AdaptiveSyncStateExt::Always),
                    Some(AdaptiveSyncState::Disabled) => Some(AdaptiveSyncStateExt::Disabled),
                    Some(_) | None => None,
                };
            }

            _ => tracing::debug!(?event, "unknown event"),
        }
    }

    event_created_child!(Context, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, Mutex::new(None)),
    ]);
}

impl Dispatch<ZcosmicOutputHeadV1, ObjectId> for Context {
    fn event(
        state: &mut Self,
        _proxy: &ZcosmicOutputHeadV1,
        event: <ZcosmicOutputHeadV1 as Proxy>::Event,
        data: &ObjectId,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        let head = state
            .output_heads
            .get_mut(data)
            .expect("Inert CosmicOutputHead");

        match event {
            ZcosmicOutputHeadEvent::Scale1000 { scale_1000 } => {
                head.scale = (scale_1000 as f64) / 1000.0;
            }
            ZcosmicOutputHeadEvent::Mirroring { name } => {
                head.mirroring = name;
            }
            ZcosmicOutputHeadEvent::AdaptiveSyncAvailable { available } => {
                head.adaptive_sync_support = Some(
                    available
                        .into_result()
                        .unwrap_or(AdaptiveSyncAvailability::Unsupported),
                );
            }
            ZcosmicOutputHeadEvent::AdaptiveSyncExt { state } => {
                head.adaptive_sync = state.into_result().ok();
            }
            ZcosmicOutputHeadEvent::XwaylandPrimary { state } => {
                head.xwayland_primary = Some(state != 0);
            }
            _ => tracing::debug!(?event, "unknown event"),
        }
    }
}

impl OutputHead {
    #[must_use]
    pub fn new(wlr_head: ZwlrOutputHeadV1, cosmic_head: Option<ZcosmicOutputHeadV1>) -> Self {
        Self {
            adaptive_sync: None,
            adaptive_sync_support: None,
            current_mode: None,
            description: String::new(),
            enabled: false,
            make: String::new(),
            model: String::new(),
            modes: IndexMap::new(),
            name: String::new(),
            physical_height: 0,
            physical_width: 0,
            position_x: 0,
            position_y: 0,
            scale: 1.0,
            serial_number: String::new(),
            transform: None,
            mirroring: None,
            xwayland_primary: None,
            wlr_head,
            cosmic_head,
        }
    }
}
