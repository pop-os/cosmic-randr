// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::fmt::Display;

use kdl::{KdlDocument, KdlError};
use slotmap::SlotMap;

slotmap::new_key_type! {
    /// A unique slotmap key to an output.
    pub struct OutputKey;
    /// A unique slotmap key to a mode.
    pub struct ModeKey;
}

#[derive(Clone, Debug)]
pub struct Mode {
    pub size: (u32, u32),
    pub refresh_rate: u32,
    pub preferred: bool,
}

impl Mode {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            size: (0, 0),
            refresh_rate: 0,
            preferred: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct List {
    pub outputs: SlotMap<OutputKey, Output>,
    pub modes: SlotMap<ModeKey, Mode>,
}

#[derive(Clone, Debug)]
pub struct Output {
    pub name: String,
    pub enabled: bool,
    pub mirroring: Option<String>,
    pub make: Option<String>,
    pub model: String,
    pub physical: (u32, u32),
    pub position: (i32, i32),
    pub scale: f64,
    pub transform: Option<Transform>,
    pub modes: Vec<ModeKey>,
    pub current: Option<ModeKey>,
    pub adaptive_sync: Option<AdaptiveSyncState>,
    pub adaptive_sync_availability: Option<AdaptiveSyncAvailability>,
}

impl Output {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name: String::new(),
            enabled: false,
            mirroring: None,
            make: None,
            model: String::new(),
            physical: (0, 0),
            position: (0, 0),
            scale: 1.0,
            transform: None,
            modes: Vec::new(),
            current: None,
            adaptive_sync: None,
            adaptive_sync_availability: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Transform {
    Normal,
    Rotate90,
    Rotate180,
    Rotate270,
    Flipped,
    Flipped90,
    Flipped180,
    Flipped270,
}

impl Display for Transform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Transform::Normal => "normal",
            Transform::Rotate90 => "rotate90",
            Transform::Rotate180 => "rotate180",
            Transform::Rotate270 => "rotate270",
            Transform::Flipped => "flipped",
            Transform::Flipped90 => "flipped90",
            Transform::Flipped180 => "flipped180",
            Transform::Flipped270 => "flipped270",
        })
    }
}

impl TryFrom<&str> for Transform {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "normal" => Transform::Normal,
            "rotate90" => Transform::Rotate90,
            "rotate180" => Transform::Rotate180,
            "rotate270" => Transform::Rotate270,
            "flipped" => Transform::Flipped,
            "flipped90" => Transform::Flipped90,
            "flipped180" => Transform::Flipped180,
            "flipped270" => Transform::Flipped270,
            _ => return Err("unknown transform variant"),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdaptiveSyncState {
    Always,
    Auto,
    Disabled,
}

impl Display for AdaptiveSyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AdaptiveSyncState::Always => "true",
            AdaptiveSyncState::Auto => "automatic",
            AdaptiveSyncState::Disabled => "false",
        })
    }
}

impl TryFrom<&str> for AdaptiveSyncState {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "true" => AdaptiveSyncState::Always,
            "automatic" => AdaptiveSyncState::Auto,
            "false" => AdaptiveSyncState::Disabled,
            _ => return Err("unknown adaptive_sync state variant"),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdaptiveSyncAvailability {
    Supported,
    RequiresModeset,
    Unsupported,
}

impl Display for AdaptiveSyncAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AdaptiveSyncAvailability::Supported => "true",
            AdaptiveSyncAvailability::RequiresModeset => "requires_modeset",
            AdaptiveSyncAvailability::Unsupported => "false",
        })
    }
}

impl TryFrom<&str> for AdaptiveSyncAvailability {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "true" => AdaptiveSyncAvailability::Supported,
            "requires_modeset" => AdaptiveSyncAvailability::RequiresModeset,
            "false" => AdaptiveSyncAvailability::Unsupported,
            _ => return Err("unknown adaptive_sync availability variant"),
        })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("`cosmic-randr` KDL format error")]
    Kdl(#[from] KdlError),
    #[error("could not exec `cosmic-randr`")]
    Spawn(#[source] std::io::Error),
    #[error("`cosmic-randr` output not UTF-8")]
    Utf(#[from] std::str::Utf8Error),
}

#[allow(clippy::too_many_lines)]
pub async fn list() -> Result<List, Error> {
    // Get a list of outputs from `cosmic-randr` in KDL format.
    let stdout = std::process::Command::new("cosmic-randr")
        .args(&["list", "--kdl"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .map_err(Error::Spawn)?
        .stdout;

    // Parse the output as a KDL document.
    let document = std::str::from_utf8(&stdout)
        .map_err(Error::Utf)?
        .parse::<KdlDocument>()
        .map_err(Error::Kdl)?;

    let mut outputs = List {
        outputs: SlotMap::with_key(),
        modes: SlotMap::with_key(),
    };

    // Each node in the root of the document is an output.
    for node in document.nodes() {
        if node.name().value() != "output" {
            eprintln!("not output");
            continue;
        }

        // Parse the properties of the output mode
        let mut entries = node.entries().iter();

        // The first value is the name of the otuput
        let Some(name) = entries.next().and_then(|e| e.value().as_string()) else {
            eprintln!("no name value");
            continue;
        };

        let mut output = Output::new();

        // Check if the output contains the `enabled` attribute.
        for entry in entries {
            let Some(entry_name) = entry.name() else {
                continue;
            };

            if entry_name.value() == "enabled" {
                if let Some(enabled) = entry.value().as_bool() {
                    output.enabled = enabled;
                }
            }
        }

        // Gets the properties of the output.
        let Some(children) = node.children() else {
            continue;
        };

        for node in children.nodes() {
            match node.name().value() {
                // Parse the make and model of the display output.
                "description" => {
                    for entry in node.entries() {
                        let value = entry.value().as_string();

                        match entry.name().map(kdl::KdlIdentifier::value) {
                            Some("make") => {
                                output.make = value.map(String::from);
                            }

                            Some("model") => {
                                if let Some(model) = value {
                                    output.model = String::from(model);
                                }
                            }

                            _ => (),
                        }
                    }
                }

                // Parse the physical width and height in millimeters
                "physical" => {
                    if let [width, height, ..] = node.entries() {
                        output.physical = (
                            width.value().as_i64().unwrap_or_default() as u32,
                            height.value().as_i64().unwrap_or_default() as u32,
                        );
                    }
                }

                // Parse the pixel coordinates of the output.
                "position" => {
                    if let [x_pos, y_pos, ..] = node.entries() {
                        output.position = (
                            x_pos.value().as_i64().unwrap_or_default() as i32,
                            y_pos.value().as_i64().unwrap_or_default() as i32,
                        );
                    }
                }

                "scale" => {
                    if let Some(entry) = node.entries().first() {
                        if let Some(scale) = entry.value().as_f64() {
                            output.scale = scale;
                        }
                    }
                }

                // Parse the transform value of the output.
                "transform" => {
                    if let Some(entry) = node.entries().first() {
                        if let Some(string) = entry.value().as_string() {
                            output.transform = Transform::try_from(string).ok();
                        }
                    }
                }

                "adaptive_sync" => {
                    if let Some(entry) = node.entries().first() {
                        if let Some(string) = entry.value().as_string() {
                            output.adaptive_sync = AdaptiveSyncState::try_from(string).ok();
                        }
                    }
                }

                "adaptive_sync_support" => {
                    if let Some(entry) = node.entries().first() {
                        if let Some(string) = entry.value().as_string() {
                            output.adaptive_sync_availability =
                                AdaptiveSyncAvailability::try_from(string).ok();
                        }
                    }
                }

                // Switch to parsing output modes.
                "modes" => {
                    let Some(children) = node.children() else {
                        continue;
                    };

                    for node in children.nodes() {
                        if node.name().value() == "mode" {
                            let mut current = false;
                            let mut mode = Mode::new();

                            if let [width, height, refresh, ..] = node.entries() {
                                mode.size = (
                                    width.value().as_i64().unwrap_or_default() as u32,
                                    height.value().as_i64().unwrap_or_default() as u32,
                                );

                                mode.refresh_rate =
                                    refresh.value().as_i64().unwrap_or_default() as u32;
                            };

                            for entry in node.entries().iter().skip(3) {
                                match entry.name().map(kdl::KdlIdentifier::value) {
                                    Some("current") => current = true,
                                    Some("preferred") => mode.preferred = true,
                                    _ => (),
                                }
                            }

                            let mode_id = outputs.modes.insert(mode);

                            if current {
                                output.current = Some(mode_id);
                            }

                            output.modes.push(mode_id);
                        }
                    }
                }

                "mirroring" => {
                    if let Some(entry) = node.entries().first() {
                        if let Some(string) = entry.value().as_string() {
                            output.mirroring = Some(string.to_string());
                        }
                    }
                }

                _ => (),
            }
        }

        output.name = name.to_owned();

        outputs.outputs.insert(output);
    }

    Ok(outputs)
}
