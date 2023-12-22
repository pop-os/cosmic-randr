// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::output_head::OutputHead;
use crate::output_mode::OutputMode;
use crate::{Error, Message};
use std::collections::HashMap;
use tachyonix::Sender;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{backend::ObjectId, Connection, Proxy, QueueHandle};
use wayland_client::{DispatchError, EventQueue};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Debug)]
pub struct Context {
    pub data: Data,
    pub connection: Connection,
    pub handle: QueueHandle<Context>,
    sender: Sender<Message>,
    pub output_configuration: Option<ZwlrOutputConfigurationV1>,
    pub output_manager: Option<ZwlrOutputManagerV1>,
    pub output_manager_serial: u32,
    pub output_manager_version: u32,
    pub output_heads: HashMap<ObjectId, OutputHead>,
    pub output_modes: HashMap<ObjectId, OutputMode>,
    pub mode_to_head_ids: HashMap<ObjectId, ObjectId>,
    pub wl_registry: WlRegistry,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct Data;

impl Context {
    pub fn callback(
        &mut self,
        event_queue: &mut EventQueue<Context>,
    ) -> Result<usize, DispatchError> {
        event_queue.dispatch_pending(self)
    }

    pub async fn dispatch(&mut self, event_queue: &mut EventQueue<Self>) -> Result<usize, Error> {
        crate::async_dispatch(&self.connection.clone(), event_queue, self)
            .await
            .map_err(Error::from)
    }

    pub async fn send(&mut self, event: Message) -> Result<(), tachyonix::SendError<Message>> {
        self.sender.send(event).await
    }

    pub fn create_output_config(&mut self) -> ZwlrOutputConfigurationV1 {
        self.destroy_output_config();
        let configuration = self.output_manager.as_ref().unwrap().create_configuration(
            self.output_manager_serial,
            &self.handle,
            self.data,
        );
        self.output_configuration = Some(configuration.clone());
        configuration
    }

    pub fn destroy_output_config(&mut self) {
        if let Some(config) = &self.output_configuration {
            config.destroy();
            self.output_configuration = None;
        }
    }

    pub(crate) fn remove_mode(&mut self, id: &ObjectId) -> Result<(), Error> {
        let head_id = self
            .mode_to_head_ids
            .remove(&id)
            .ok_or(Error::ReleaseOutputMode)?;

        let head = self
            .output_heads
            .get_mut(&head_id)
            .ok_or(Error::ReleaseOutputMode)?;

        if let Some(mode_id) = &head.current_mode {
            if mode_id == id {
                head.current_mode = None;
            }
        }

        head.modes.retain(|e| e != id);

        Ok(())
    }

    pub fn connect(sender: Sender<Message>) -> Result<(Self, EventQueue<Self>), Error> {
        let connection = Connection::connect_to_env()?;
        let data = Data::default();

        let event_queue = connection.new_event_queue();
        let handle = event_queue.handle();

        let display = connection.display();
        let wl_registry = display.get_registry(&handle, data);

        let context = Self {
            connection,
            handle,
            data,
            output_manager_serial: Default::default(),
            output_manager: Default::default(),
            output_manager_version: Default::default(),
            output_configuration: Default::default(),
            output_heads: Default::default(),
            output_modes: Default::default(),
            mode_to_head_ids: Default::default(),
            sender,
            wl_registry,
        };

        Ok((context, event_queue))
    }

    /// Flushes the wayland client connection.
    ///
    /// # Errors
    ///
    /// Returns error if wayland client connection fails to flush.
    pub fn flush(&mut self) -> Result<(), Error> {
        Ok(self.connection.flush()?)
    }

    pub fn clear(&mut self) {
        self.destroy_output_config();

        for (id, _) in std::mem::take(&mut self.output_modes) {
            match ZwlrOutputModeV1::from_id(&self.connection, id) {
                Ok(it) => it.release(),
                Err(err) => tracing::debug!("{}", err),
            }
        }

        for (id, _) in std::mem::take(&mut self.output_heads) {
            match ZwlrOutputHeadV1::from_id(&self.connection, id) {
                Ok(it) => it.release(),
                Err(err) => tracing::debug!("{}", err),
            }
        }

        if let Some(manager) = &self.output_manager {
            manager.stop();
        }
    }
}
