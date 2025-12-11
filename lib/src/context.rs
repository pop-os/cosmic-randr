// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use crate::output_head::OutputHead;
use crate::{Error, Message, Sender};
use cosmic_protocols::output_management::v1::client::zcosmic_output_configuration_head_v1::{
    self, ZcosmicOutputConfigurationHeadV1,
};
use cosmic_protocols::output_management::v1::client::zcosmic_output_configuration_v1::ZcosmicOutputConfigurationV1;
use cosmic_protocols::output_management::v1::client::zcosmic_output_head_v1::AdaptiveSyncStateExt;
use cosmic_protocols::output_management::v1::client::zcosmic_output_manager_v1::{
    self, ZcosmicOutputManagerV1,
};
use std::collections::HashMap;
use std::fmt;
use wayland_client::protocol::{
    wl_callback::WlCallback, wl_output::Transform, wl_registry::WlRegistry,
};
use wayland_client::{Connection, Proxy, QueueHandle, backend::ObjectId};
use wayland_client::{DispatchError, EventQueue};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::{
    AdaptiveSyncState, ZwlrOutputHeadV1,
};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;

pub struct Context {
    pub connection: Connection,
    pub handle: QueueHandle<Context>,
    sender: Sender,

    pub output_manager: Option<ZwlrOutputManagerV1>,
    pub cosmic_output_manager: Option<ZcosmicOutputManagerV1>,
    pub output_manager_serial: u32,
    pub output_manager_version: u32,

    pub output_heads: HashMap<ObjectId, OutputHead>,
    pub wl_registry: WlRegistry,

    pub cosmic_manager_sync_callback: Option<WlCallback>,
    pub done_queued: bool,
}

#[derive(Debug)]
pub struct Configuration {
    obj: ZwlrOutputConfigurationV1,
    cosmic_obj: Option<ZcosmicOutputConfigurationV1>,
    cosmic_output_manager: Option<ZcosmicOutputManagerV1>,
    handle: QueueHandle<Context>,

    known_heads: Vec<OutputHead>,
    configured_heads: Vec<String>,
}

#[derive(Debug, Default)]
pub struct HeadConfiguration {
    /// Specifies the width and height of the output picture.
    pub size: Option<(u32, u32)>,
    /// Specifies the refresh rate to apply to the output.
    pub refresh: Option<f32>,
    /// Specifies the adaptive_sync mode to apply to the output.
    pub adaptive_sync: Option<AdaptiveSyncStateExt>,
    /// Position the output within this x pixel coordinate.
    pub pos: Option<(i32, i32)>,
    /// Changes the dimensions of the output picture.
    pub scale: Option<f64>,
    /// Specifies a transformation matrix to apply to the output.
    pub transform: Option<Transform>,
}

#[derive(Debug, Clone, Copy)]
pub enum ConfigurationError {
    OutputAlreadyConfigured,
    UnknownOutput,
    ModeNotFound,
    NoCosmicExtension,
    PositionForMirroredOutput,
    MirroringItself,
    UnsupportedVrrState,
    UnsupportedXwaylandPrimary,
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputAlreadyConfigured => f.write_str("Output configured twice"),
            Self::UnknownOutput => f.write_str("Unknown output"),
            Self::ModeNotFound => f.write_str("Unknown or unsupported mode"),
            Self::NoCosmicExtension => f.write_str("Feature isn't available outside COSMIC"),
            Self::PositionForMirroredOutput => f.write_str("You cannot position a mirrored output"),
            Self::MirroringItself => f.write_str("Output mirroring itself"),
            Self::UnsupportedVrrState => {
                f.write_str("Automatic VRR state management isn't available outside COSMIC")
            }
            Self::UnsupportedXwaylandPrimary => f.write_str(
                "Xwayland compatibility options not available outside or on this version of COSMIC",
            ),
        }
    }
}
impl std::error::Error for ConfigurationError {}

impl Configuration {
    pub fn disable_head(&mut self, output: &str) -> Result<(), ConfigurationError> {
        if self.configured_heads.iter().any(|o| o == output) {
            return Err(ConfigurationError::OutputAlreadyConfigured);
        }
        self.configured_heads.push(output.to_string());

        let head = self
            .known_heads
            .iter()
            .find(|head| head.name == output)
            .ok_or(ConfigurationError::UnknownOutput)?;
        self.obj.disable_head(&head.wlr_head);

        Ok(())
    }

    pub fn enable_head(
        &mut self,
        output: &str,
        mode: Option<HeadConfiguration>,
    ) -> Result<(), ConfigurationError> {
        if self.configured_heads.iter().any(|o| o == output) {
            return Err(ConfigurationError::OutputAlreadyConfigured);
        }
        self.configured_heads.push(output.to_string());

        let head = self
            .known_heads
            .iter()
            .find(|head| head.name == output)
            .ok_or(ConfigurationError::UnknownOutput)?;
        let head_config = self.obj.enable_head(&head.wlr_head, &self.handle, ());
        let cosmic_head_config = self
            .cosmic_output_manager
            .as_ref()
            .map(|extension| extension.get_configuration_head(&head_config, &self.handle, ()));

        if let Some(args) = mode {
            send_mode_to_config_head(head, head_config, cosmic_head_config, args)?;
        }

        Ok(())
    }

    pub fn mirror_head(
        &mut self,
        output: &str,
        mirrored: &str,
        mode: Option<HeadConfiguration>,
    ) -> Result<(), ConfigurationError> {
        if self.cosmic_obj.is_none() {
            return Err(ConfigurationError::NoCosmicExtension);
        }

        if self.configured_heads.iter().any(|o| o == output) {
            return Err(ConfigurationError::OutputAlreadyConfigured);
        }

        if output == mirrored {
            return Err(ConfigurationError::MirroringItself);
        }

        if mode.as_ref().is_some_and(|mode| mode.pos.is_some()) {
            return Err(ConfigurationError::PositionForMirroredOutput);
        }

        self.configured_heads.push(output.to_string());

        let head = self
            .known_heads
            .iter()
            .find(|head| head.name == output)
            .ok_or(ConfigurationError::UnknownOutput)?;
        let mirror_head = self
            .known_heads
            .iter()
            .find(|head| head.name == mirrored)
            .ok_or(ConfigurationError::UnknownOutput)?;

        let cosmic_obj = self.cosmic_obj.as_ref().unwrap();
        let head_config =
            cosmic_obj.mirror_head(&head.wlr_head, &mirror_head.wlr_head, &self.handle, ());
        let cosmic_head_config = self
            .cosmic_output_manager
            .as_ref()
            .map(|extension| extension.get_configuration_head(&head_config, &self.handle, ()));

        if let Some(args) = mode {
            send_mode_to_config_head(head, head_config, cosmic_head_config, args)?;
        }

        Ok(())
    }

    fn configure_remaining_heads(&mut self) {
        let known_heads = self.known_heads.clone();
        let configured_heads = self.configured_heads.clone();
        for output in known_heads
            .iter()
            .filter(|output| !configured_heads.contains(&output.name))
        {
            if output.enabled {
                if let Some(from) = output.mirroring.as_ref() {
                    self.mirror_head(&output.name, from, None).unwrap();
                } else {
                    self.enable_head(&output.name, None).unwrap();
                }
            } else {
                self.disable_head(&output.name).unwrap();
            }
        }
    }

    pub fn test(mut self) {
        self.configure_remaining_heads();
        self.obj.test();
    }

    pub fn apply(mut self) {
        self.configure_remaining_heads();
        self.obj.apply();
    }

    pub fn cancel(self) {
        self.obj.destroy()
    }
}

fn send_mode_to_config_head(
    head: &OutputHead,
    head_config: ZwlrOutputConfigurationHeadV1,
    cosmic_head_config: Option<ZcosmicOutputConfigurationHeadV1>,
    args: HeadConfiguration,
) -> Result<(), ConfigurationError> {
    if let Some(scale) = args.scale {
        if let Some(cosmic_obj) = cosmic_head_config.as_ref() {
            cosmic_obj.set_scale_1000((scale * 1000.0) as i32);
        } else {
            head_config.set_scale(scale);
        }
    }

    if let Some(transform) = args.transform {
        head_config.set_transform(transform);
    }

    if let Some((x, y)) = args.pos {
        head_config.set_position(x, y);
    }

    let mode_iter = || {
        head.modes.values().filter(|mode| {
            if let Some((width, height)) = args.size {
                mode.width == width as i32 && mode.height == height as i32
            } else {
                head.current_mode
                    .as_ref()
                    .is_some_and(|current_mode| mode.wlr_mode.id() == *current_mode)
            }
        })
    };

    if let Some(vrr) = args.adaptive_sync {
        if let Some(cosmic_obj) = cosmic_head_config.as_ref().filter(|obj| {
            obj.version() >= zcosmic_output_configuration_head_v1::REQ_SET_ADAPTIVE_SYNC_EXT_SINCE
        }) {
            cosmic_obj.set_adaptive_sync_ext(vrr);
        } else {
            head_config.set_adaptive_sync(match vrr {
                AdaptiveSyncStateExt::Always => AdaptiveSyncState::Enabled,
                AdaptiveSyncStateExt::Disabled => AdaptiveSyncState::Disabled,
                AdaptiveSyncStateExt::Automatic => {
                    return Err(ConfigurationError::UnsupportedVrrState);
                }
                _ => panic!("Unknown AdaptiveSyncStatExt variant"),
            });
        }
    }

    if let Some(refresh) = args.refresh {
        #[allow(clippy::cast_possible_truncation)]
        let refresh = (refresh * 1000.0) as i32;

        let min = refresh - 501;
        let max = refresh + 501;

        let mode = mode_iter()
            .find(|mode| mode.refresh == refresh)
            .or_else(|| {
                mode_iter()
                    .filter(|mode| min < mode.refresh && max > mode.refresh)
                    .min_by_key(|mode| (mode.refresh - refresh).abs())
            });

        if let Some(mode) = mode {
            head_config.set_mode(&mode.wlr_mode);
            Ok(())
        } else {
            Err(ConfigurationError::ModeNotFound)
        }
    } else if let Some(mode) = mode_iter().next() {
        head_config.set_mode(&mode.wlr_mode);
        Ok(())
    } else {
        Err(ConfigurationError::ModeNotFound)
    }
}

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

    pub fn send(&mut self, event: Message) {
        self.sender.send(event)
    }

    pub fn create_output_config(&mut self) -> Configuration {
        let configuration = self.output_manager.as_ref().unwrap().create_configuration(
            self.output_manager_serial,
            &self.handle,
            (),
        );

        let cosmic_configuration = self
            .cosmic_output_manager
            .as_ref()
            .map(|extension| extension.get_configuration(&configuration, &self.handle, ()));

        Configuration {
            obj: configuration,
            cosmic_obj: cosmic_configuration,
            cosmic_output_manager: self.cosmic_output_manager.clone(),
            handle: self.handle.clone(),
            known_heads: self.output_heads.values().cloned().collect(),
            configured_heads: Vec::new(),
        }
    }

    pub fn set_xwayland_primary(&self, output: Option<&str>) -> Result<(), ConfigurationError> {
        let Some(cosmic_output_manager) = self.cosmic_output_manager.as_ref() else {
            return Err(ConfigurationError::NoCosmicExtension);
        };
        if cosmic_output_manager.version()
            < zcosmic_output_manager_v1::REQ_SET_XWAYLAND_PRIMARY_SINCE
        {
            return Err(ConfigurationError::UnsupportedXwaylandPrimary);
        }

        match output {
            None => cosmic_output_manager.set_xwayland_primary(None),
            Some(name) => {
                let head = self
                    .output_heads
                    .values()
                    .filter(|head| head.cosmic_head.is_some())
                    .find(|head| head.name == name)
                    .ok_or(ConfigurationError::UnknownOutput)?;
                cosmic_output_manager.set_xwayland_primary(Some(head.cosmic_head.as_ref().unwrap()))
            }
        };

        Ok(())
    }

    pub fn connect(sender: Sender) -> Result<(Self, EventQueue<Self>), Error> {
        let connection = Connection::connect_to_env()?;

        let mut event_queue = connection.new_event_queue();
        let handle = event_queue.handle();

        let display = connection.display();
        let wl_registry = display.get_registry(&handle, ());

        let mut context = Self {
            connection,
            handle,
            output_manager_serial: Default::default(),
            output_manager: Default::default(),
            cosmic_output_manager: Default::default(),
            output_manager_version: Default::default(),
            output_heads: Default::default(),
            sender,
            wl_registry,
            cosmic_manager_sync_callback: None,
            done_queued: false,
        };

        event_queue.roundtrip(&mut context)?;
        // second roundtrip for extension protocol
        if context.cosmic_output_manager.is_some() {
            event_queue.roundtrip(&mut context)?;
        }

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

    pub async fn apply_current_config(&mut self) -> Result<(), ConfigurationError> {
        let Some(cosmic_output_manager) = self.cosmic_output_manager.as_ref() else {
            return Err(ConfigurationError::NoCosmicExtension);
        };
        if cosmic_output_manager.version()
            < zcosmic_output_manager_v1::REQ_SET_XWAYLAND_PRIMARY_SINCE
        {
            return Err(ConfigurationError::UnsupportedXwaylandPrimary);
        }

        let configuration = self.output_manager.as_ref().unwrap().create_configuration(
            self.output_manager_serial,
            &self.handle,
            (),
        );

        let cosmic_configuration = self
            .cosmic_output_manager
            .as_ref()
            .map(|extension| extension.get_configuration(&configuration, &self.handle, ()));

        let mut config_obj = Configuration {
            obj: configuration,
            cosmic_obj: cosmic_configuration,
            cosmic_output_manager: self.cosmic_output_manager.clone(),
            handle: self.handle.clone(),
            known_heads: self.output_heads.values().cloned().collect(),
            configured_heads: Vec::new(),
        };

        let known_heads = config_obj.known_heads.clone();
        let configured_heads = config_obj.configured_heads.clone();
        for output in known_heads
            .iter()
            .filter(|output| !configured_heads.contains(&output.name))
        {
            let head_configuration = HeadConfiguration {
                size: output.current_mode.as_ref().and_then(|mode_id| {
                    output
                        .modes
                        .get(mode_id)
                        .map(|mode| (mode.width as u32, mode.height as u32))
                }),
                refresh: output.current_mode.as_ref().and_then(|mode_id| {
                    output
                        .modes
                        .get(mode_id)
                        .map(|mode| mode.refresh as f32 / 1000.0)
                }),
                adaptive_sync: output.adaptive_sync,
                pos: Some((output.position_x, output.position_y)),
                scale: Some(output.scale),
                transform: output.transform,
            };
            if output.enabled {
                if let Some(from) = output.mirroring.as_ref() {
                    config_obj
                        .mirror_head(&output.name, from, Some(head_configuration))
                        .unwrap();
                } else {
                    config_obj
                        .enable_head(&output.name, Some(head_configuration))
                        .unwrap();
                }
            } else {
                config_obj.disable_head(&output.name).unwrap();
            }
        }
        config_obj.apply();

        Ok(())
    }
}
