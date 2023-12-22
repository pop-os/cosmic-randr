// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::{Context, Data, OutputMode};

use wayland_client::backend::ObjectId;
use wayland_client::event_created_child;
use wayland_client::protocol::wl_output::Transform;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::AdaptiveSyncState;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::Event as ZwlrOutputHeadEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::EVT_MODE_OPCODE;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Clone, Debug, PartialEq)]
pub struct OutputHead {
    pub adaptive_sync: Option<AdaptiveSyncState>,
    pub current_mode: Option<ObjectId>,
    pub description: String,
    pub enabled: bool,
    pub make: String,
    pub model: String,
    pub modes: Vec<ObjectId>,
    pub name: String,
    pub physical_height: i32,
    pub physical_width: i32,
    pub position_x: i32,
    pub position_y: i32,
    pub scale: f64,
    pub serial_number: String,
    pub transform: Option<Transform>,
    pub wlr_head: ZwlrOutputHeadV1,
}

impl Dispatch<ZwlrOutputHeadV1, Data> for Context {
    fn event(
        state: &mut Self,
        proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _: &Data,
        _: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        let head = state
            .output_heads
            .entry(proxy.id())
            .or_insert_with(|| OutputHead::new(proxy.clone()));

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
                let mode_id = mode.id();
                head.modes.push(mode_id.clone());
                state.mode_to_head_ids.insert(mode_id.clone(), proxy.id());
                state.output_modes.insert(mode_id, OutputMode::new(mode));
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
                proxy.release();
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
                head.adaptive_sync = state.into_result().ok();
            }

            _ => tracing::debug!(?event, "unknown event"),
        }
    }

    event_created_child!(Context, ZwlrOutputManagerV1, [
        EVT_MODE_OPCODE => (ZwlrOutputModeV1, Data),
    ]);
}

impl OutputHead {
    #[must_use]
    pub fn new(wlr_head: ZwlrOutputHeadV1) -> Self {
        Self {
            adaptive_sync: None,
            current_mode: None,
            description: String::new(),
            enabled: false,
            make: String::new(),
            model: String::new(),
            modes: Vec::new(),
            name: String::new(),
            physical_height: 0,
            physical_width: 0,
            position_x: 0,
            position_y: 0,
            scale: 1.0,
            serial_number: String::new(),
            transform: None,
            wlr_head,
        }
    }
}
