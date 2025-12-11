// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::output_head::OutputHead;
use crate::{Context, Message};

use cosmic_protocols::output_management::v1::client::zcosmic_output_manager_v1::ZcosmicOutputManagerV1;
use wayland_client::event_created_child;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::EVT_HEAD_OPCODE;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as ZwlrOutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

impl Dispatch<ZwlrOutputManagerV1, ()> for Context {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _data: &(),
        conn: &Connection,
        handle: &QueueHandle<Self>,
    ) {
        match event {
            ZwlrOutputManagerEvent::Head { head } => {
                let cosmic_head =
                    if let Some(cosmic_extension) = state.cosmic_output_manager.as_ref() {
                        let cosmic_head = cosmic_extension.get_head(&head, handle, head.id());

                        // Use `sync` callback to wait until `get_head` is processed and
                        // we also have cosmic extension events.
                        let callback = conn.display().sync(handle, ());
                        state.cosmic_manager_sync_callback = Some(callback);
                        Some(cosmic_head)
                    } else {
                        None
                    };
                state
                    .output_heads
                    .insert(head.id(), OutputHead::new(head, cosmic_head));
            }

            ZwlrOutputManagerEvent::Done { serial } => {
                state.output_manager_serial = serial;
                if state.cosmic_manager_sync_callback.is_some() {
                    // Potentally waiting for cosmic extension events after calling
                    // `get_head`. Queue sending `ManagerDone` until sync callback.
                    state.done_queued = true;
                } else {
                    let _res = state.send(Message::ManagerDone);
                }
            }

            ZwlrOutputManagerEvent::Finished => {
                state.output_manager = None;
                state.output_manager_serial = 0;
                let _res = state.send(Message::ManagerFinished);
            }

            _ => tracing::debug!(?event, "unknown event"),
        }
    }

    event_created_child!(Context, ZwlrOutputManagerV1, [
        EVT_HEAD_OPCODE=> (ZwlrOutputHeadV1, ()),
    ]);
}

impl Dispatch<ZcosmicOutputManagerV1, ()> for Context {
    fn event(
        _state: &mut Self,
        _proxy: &ZcosmicOutputManagerV1,
        _event: <ZcosmicOutputManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
    }
}

use wayland_client::protocol::wl_callback::{self, WlCallback};

impl Dispatch<WlCallback, ()> for Context {
    fn event(
        state: &mut Self,
        proxy: &WlCallback,
        event: wl_callback::Event,
        _data: &(),
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        match event {
            wl_callback::Event::Done { callback_data: _ } => {
                if state.cosmic_manager_sync_callback.as_ref() == Some(proxy) {
                    state.cosmic_manager_sync_callback = None;
                    if state.done_queued {
                        let _res = state.send(Message::ManagerDone);
                        state.done_queued = false;
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}
