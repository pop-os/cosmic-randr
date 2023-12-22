// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use super::output_head::OutputHead;
use crate::{Context, Data, Message};

use wayland_client::event_created_child;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::Event as ZwlrOutputManagerEvent;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::EVT_HEAD_OPCODE;

impl Dispatch<ZwlrOutputManagerV1, Data> for Context {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _handle: &QueueHandle<Self>,
    ) {
        match event {
            ZwlrOutputManagerEvent::Head { head } => {
                state.output_heads.insert(head.id(), OutputHead::new(head));
            }

            ZwlrOutputManagerEvent::Done { serial } => {
                state.output_manager_serial = serial;
                futures_lite::future::block_on(async {
                    let _res = state.send(Message::ManagerDone).await;
                });
            }

            ZwlrOutputManagerEvent::Finished => {
                state.output_manager = None;
                state.output_manager_serial = 0;
                futures_lite::future::block_on(async {
                    let _res = state.send(Message::ManagerFinished).await;
                });
            }

            _ => tracing::debug!(?event, "unknown event"),
        }
    }

    event_created_child!(Context, ZwlrOutputManagerV1, [
        EVT_HEAD_OPCODE=> (ZwlrOutputHeadV1, Data),
    ]);
}
