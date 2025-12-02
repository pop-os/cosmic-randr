// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use std::fmt::Display;

use kdl::{KdlDocument, KdlEntry, KdlError, KdlValue};
use slotmap::SlotMap;

slotmap::new_key_type! {
    /// A unique slotmap key to an output.
    pub struct OutputKey;
    /// A unique slotmap key to a mode.
    pub struct ModeKey;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mode {
    pub size: (u32, u32),
    pub refresh_rate: u32,
    pub preferred: bool,
}

impl Default for Mode {
    fn default() -> Self {
        Self::new()
    }
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

#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub serial_number: String,
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
    pub xwayland_primary: Option<bool>,
}

impl Default for Output {
    fn default() -> Self {
        Self::new()
    }
}

impl Output {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            serial_number: String::new(),
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
            xwayland_primary: None,
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

impl AdaptiveSyncState {
    fn try_from_kdl_value(value: &KdlValue) -> Option<Self> {
        value.as_bool().map_or_else(
            || {
                value
                    .as_string()
                    .and_then(|v| AdaptiveSyncState::try_from(v).ok())
            },
            |v| {
                Some(if v {
                    AdaptiveSyncState::Always
                } else {
                    AdaptiveSyncState::Disabled
                })
            },
        )
    }
}

impl From<AdaptiveSyncState> for KdlValue {
    fn from(this: AdaptiveSyncState) -> Self {
        match this {
            AdaptiveSyncState::Disabled => KdlValue::Bool(false),
            AdaptiveSyncState::Always => KdlValue::Bool(true),
            AdaptiveSyncState::Auto => KdlValue::String("automatic".into()),
        }
    }
}

impl TryFrom<&str> for AdaptiveSyncState {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "automatic" => AdaptiveSyncState::Auto,
            _ => return Err("unknown adaptive_sync state variant"),
        })
    }
}

impl From<AdaptiveSyncState> for &'static str {
    fn from(this: AdaptiveSyncState) -> Self {
        match this {
            AdaptiveSyncState::Always => "true",
            AdaptiveSyncState::Auto => "automatic",
            AdaptiveSyncState::Disabled => "false",
        }
    }
}

impl Display for AdaptiveSyncState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&'static str>::from(*self))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdaptiveSyncAvailability {
    Supported,
    RequiresModeset,
    Unsupported,
}

impl AdaptiveSyncAvailability {
    pub fn try_from_kdl_value(value: &KdlValue) -> Option<Self> {
        value.as_bool().map_or_else(
            || {
                value
                    .as_string()
                    .and_then(|v| AdaptiveSyncAvailability::try_from(v).ok())
            },
            |v| {
                Some(if v {
                    AdaptiveSyncAvailability::Supported
                } else {
                    AdaptiveSyncAvailability::Unsupported
                })
            },
        )
    }
}

impl From<AdaptiveSyncAvailability> for KdlValue {
    fn from(this: AdaptiveSyncAvailability) -> Self {
        match this {
            AdaptiveSyncAvailability::Unsupported => KdlValue::Bool(false),
            AdaptiveSyncAvailability::Supported => KdlValue::Bool(true),
            AdaptiveSyncAvailability::RequiresModeset => {
                KdlValue::String("requires_modeset".into())
            }
        }
    }
}

impl TryFrom<&str> for AdaptiveSyncAvailability {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "requires_modeset" => AdaptiveSyncAvailability::RequiresModeset,
            _ => return Err("unknown adaptive_sync availability variant"),
        })
    }
}

impl From<AdaptiveSyncAvailability> for &'static str {
    fn from(this: AdaptiveSyncAvailability) -> Self {
        match this {
            AdaptiveSyncAvailability::Supported => "true",
            AdaptiveSyncAvailability::RequiresModeset => "requires_modeset",
            AdaptiveSyncAvailability::Unsupported => "false",
        }
    }
}

impl Display for AdaptiveSyncAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&'static str>::from(*self))
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
        .args(["list", "--kdl"])
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

    match List::try_from(document) {
        Ok(v) => Ok(v),
        Err(KdlParseWithError { list, errors }) => {
            for err in errors {
                eprintln!("{err:?}");
            }
            Ok(list)
        }
    }
}

#[derive(Debug, Clone)]
pub enum KdlParseError {
    InvalidRootNode(String),
    InvalidKey(String),
    InvalidValue { key: String, value: Vec<KdlEntry> },
    MissingOutputName,
    MissingOutputChildren,
    MissingModeChildren,
    MissingEntryName,
}

#[derive(Debug, Clone)]
pub struct KdlParseWithError {
    pub list: List,
    pub errors: Vec<KdlParseError>,
}

impl TryFrom<KdlDocument> for List {
    type Error = KdlParseWithError;

    fn try_from(document: KdlDocument) -> Result<Self, Self::Error> {
        let mut outputs = List {
            outputs: SlotMap::with_key(),
            modes: SlotMap::with_key(),
        };
        let mut errors = Vec::new();

        // Each node in the root of the document is an output.
        for node in document.nodes() {
            if node.name().value() != "output" {
                errors.push(KdlParseError::InvalidRootNode(
                    node.name().value().to_string(),
                ));
                continue;
            }

            // Parse the properties of the output mode
            let mut entries = node.entries().iter();

            // The first value is the name of the otuput
            let Some(name) = entries.next().and_then(|e| e.value().as_string()) else {
                errors.push(KdlParseError::MissingOutputName);
                continue;
            };

            let mut output = Output::new();

            // Check if the output contains the `enabled` attribute.
            for entry in entries {
                let Some(entry_name) = entry.name() else {
                    errors.push(KdlParseError::MissingEntryName);
                    continue;
                };

                if entry_name.value() == "enabled"
                    && let Some(enabled) = entry.value().as_bool()
                {
                    output.enabled = enabled;
                }
            }

            // Gets the properties of the output.
            let Some(children) = node.children() else {
                errors.push(KdlParseError::MissingOutputChildren);
                continue;
            };

            for node in children.nodes() {
                match node.name().value() {
                    // Parse the serial number
                    "serial_number" => {
                        if let Some(entry) = node.entries().first() {
                            output.serial_number =
                                entry.value().as_string().unwrap_or("").to_owned();
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "serial_number".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

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

                                v => errors.push(KdlParseError::InvalidKey(
                                    v.map_or(String::default(), |s| s.to_string()),
                                )),
                            }
                        }
                    }

                    // Parse the physical width and height in millimeters
                    "physical" => {
                        if let [width, height, ..] = node.entries() {
                            output.physical = (
                                width.value().as_integer().unwrap_or_default() as u32,
                                height.value().as_integer().unwrap_or_default() as u32,
                            );
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "physical".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    // Parse the pixel coordinates of the output.
                    "position" => {
                        if let [x_pos, y_pos, ..] = node.entries() {
                            output.position = (
                                x_pos.value().as_integer().unwrap_or_default() as i32,
                                y_pos.value().as_integer().unwrap_or_default() as i32,
                            );
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "position".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    "scale" => {
                        if let Some(entry) = node.entries().first() {
                            if let Some(scale) = entry.value().as_float() {
                                output.scale = scale;
                            }
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "scale".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    // Parse the transform value of the output.
                    "transform" => {
                        if let Some(entry) = node.entries().first() {
                            if let Some(string) = entry.value().as_string() {
                                output.transform = Transform::try_from(string).ok();
                            }
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "transform".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    "adaptive_sync" => {
                        if let Some(entry) = node.entries().first() {
                            output.adaptive_sync =
                                AdaptiveSyncState::try_from_kdl_value(entry.value())
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "adaptive_sync".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    "adaptive_sync_support" => {
                        if let Some(entry) = node.entries().first() {
                            output.adaptive_sync_availability =
                                AdaptiveSyncAvailability::try_from_kdl_value(entry.value());
                        } else {
                            errors.push(KdlParseError::InvalidValue {
                                key: "adaptive_sync_support".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    // Switch to parsing output modes.
                    "modes" => {
                        let Some(children) = node.children() else {
                            errors.push(KdlParseError::MissingModeChildren);
                            continue;
                        };

                        for node in children.nodes() {
                            if node.name().value() == "mode" {
                                let mut current = false;
                                let mut mode = Mode::new();

                                if let [width, height, refresh, ..] = node.entries() {
                                    mode.size = (
                                        width.value().as_integer().unwrap_or_default() as u32,
                                        height.value().as_integer().unwrap_or_default() as u32,
                                    );

                                    mode.refresh_rate =
                                        refresh.value().as_integer().unwrap_or_default() as u32;
                                };

                                for entry in node.entries().iter().skip(3) {
                                    match entry.name().map(kdl::KdlIdentifier::value) {
                                        Some("current") => current = true,
                                        Some("preferred") => mode.preferred = true,
                                        _ => {
                                            errors.push(KdlParseError::InvalidKey(
                                                entry
                                                    .name()
                                                    .map_or(String::default(), |s| s.to_string()),
                                            ));
                                        }
                                    }
                                }

                                let mode_id = outputs.modes.insert(mode);

                                if current {
                                    output.current = Some(mode_id);
                                }

                                output.modes.push(mode_id);
                            } else {
                                errors.push(KdlParseError::InvalidKey(
                                    node.name().value().to_string(),
                                ));
                            }
                        }
                    }

                    "mirroring" => {
                        let mut applied = false;

                        if let Some(entry) = node.entries().first()
                            && let Some(string) = entry.value().as_string()
                        {
                            applied = true;
                            output.mirroring = Some(string.to_string());
                        }
                        if !applied {
                            errors.push(KdlParseError::InvalidValue {
                                key: "mirroring".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    "xwayland_primary" => {
                        let mut applied = false;
                        if let Some(entry) = node.entries().first()
                            && let Some(val) = entry.value().as_bool()
                        {
                            applied = true;
                            output.xwayland_primary = Some(val);
                        }
                        if !applied {
                            errors.push(KdlParseError::InvalidValue {
                                key: "xwayland_primary".to_string(),
                                value: node.entries().to_vec(),
                            });
                        }
                    }

                    _ => errors.push(KdlParseError::InvalidKey(node.name().value().to_string())),
                };
            }

            output.name = name.to_owned();

            outputs.outputs.insert(output);
        }
        if errors.is_empty() {
            Ok(outputs)
        } else {
            Err(KdlParseWithError {
                list: outputs,
                errors,
            })
        }
    }
}

impl From<List> for KdlDocument {
    fn from(value: List) -> Self {
        let mut doc = KdlDocument::new();

        for (_output_key, output) in value.outputs.iter() {
            let mut output_node = kdl::KdlNode::new("output");

            // Serial number (if any)
            if !output.serial_number.is_empty() {
                output_node.push(output.serial_number.clone());
            }

            // Display adapter name (unnamed)
            output_node.push(output.name.clone());

            // Additional entries: enabled (named)
            output_node.push(("enabled", output.enabled));

            // Children: description, physical, position, scale, transform, adaptive_sync, adaptive_sync_support, mirroring, xwayland_primary, modes
            let mut children = KdlDocument::new();

            // description node
            if output.make.is_some() || !output.model.is_empty() {
                let mut desc_node = kdl::KdlNode::new("description");
                if let Some(make) = &output.make {
                    desc_node.push(("make", make.clone()));
                }
                if !output.model.is_empty() {
                    desc_node.push(("model", output.model.clone()));
                }
                children.nodes_mut().push(desc_node);
            }

            // physical node
            children.nodes_mut().push({
                let mut node = kdl::KdlNode::new("physical");
                node.push(output.physical.0 as i128);
                node.push(output.physical.1 as i128);
                node
            });

            // position node
            children.nodes_mut().push({
                let mut node = kdl::KdlNode::new("position");
                node.push(output.position.0 as i128);
                node.push(output.position.1 as i128);
                node
            });

            // scale node
            children.nodes_mut().push({
                let mut node = kdl::KdlNode::new("scale");
                node.push(output.scale);
                node
            });

            // transform node
            if let Some(transform) = output.transform {
                let mut node = kdl::KdlNode::new("transform");
                node.push(transform.to_string());
                children.nodes_mut().push(node);
            }

            // adaptive_sync node
            if let Some(adaptive_sync) = output.adaptive_sync {
                let mut node = kdl::KdlNode::new("adaptive_sync");
                node.push(KdlEntry::new(adaptive_sync));
                children.nodes_mut().push(node);
            }

            if let Some(adaptive_sync_availability) = output.adaptive_sync_availability {
                let mut node = kdl::KdlNode::new("adaptive_sync_support");
                node.push(KdlEntry::new(adaptive_sync_availability));
                children.nodes_mut().push(node);
            }

            // mirroring node
            if let Some(mirroring) = &output.mirroring {
                let mut node = kdl::KdlNode::new("mirroring");
                node.push(mirroring.clone());
                children.nodes_mut().push(node);
            }

            // xwayland_primary node
            if let Some(xwayland_primary) = output.xwayland_primary {
                let mut node = kdl::KdlNode::new("xwayland_primary");
                node.push(xwayland_primary);
                children.nodes_mut().push(node);
            }

            // modes node
            let mut modes_node = kdl::KdlNode::new("modes");
            let mut modes_children = KdlDocument::new();

            for mode_key in &output.modes {
                if let Some(mode) = value.modes.get(*mode_key) {
                    let mut mode_node = kdl::KdlNode::new("mode");
                    mode_node.push(mode.size.0 as i128);
                    mode_node.push(mode.size.1 as i128);
                    mode_node.push(mode.refresh_rate as i128);

                    if output.current == Some(*mode_key) {
                        mode_node.push(("current", true));
                    }
                    if mode.preferred {
                        mode_node.push(("preferred", true));
                    }
                    modes_children.nodes_mut().push(mode_node);
                }
            }

            if !modes_children.nodes().is_empty() {
                modes_node.set_children(modes_children);
                children.nodes_mut().push(modes_node);
            }

            output_node.set_children(children);

            doc.nodes_mut().push(output_node);
        }

        doc
    }
}
#[cfg(test)]

mod test {
    use super::*;
    use kdl::KdlDocument;

    #[test]
    fn test_kdl_serialization_deserialization() {
        let mut list = List::default();

        let mode1 = Mode {
            size: (1920, 1080),
            refresh_rate: 60000,
            preferred: true,
        };
        let mode2 = Mode {
            size: (1280, 720),
            refresh_rate: 60000,
            preferred: false,
        };

        let mode1_key = list.modes.insert(mode1);
        let mode2_key = list.modes.insert(mode2);

        let output = Output {
            serial_number: String::new(),
            name: "HDMI-A-1".to_string(),
            enabled: true,
            mirroring: Some("eDP-1".to_string()),
            make: Some("Hello".to_string()),
            model: "Hi".to_string(),
            physical: (344, 194),
            position: (0, 0),
            scale: 1.0,
            transform: Some(Transform::Normal),
            modes: vec![mode1_key, mode2_key],
            current: Some(mode1_key),
            adaptive_sync: Some(AdaptiveSyncState::Auto),
            adaptive_sync_availability: Some(AdaptiveSyncAvailability::Supported),
            xwayland_primary: Some(true),
        };

        list.outputs.insert(output);

        // Serialize to KDL
        let kdl_doc: KdlDocument = list.clone().into();
        let kdl_string = kdl_doc.to_string();

        // Parse back from KDL
        let parsed_doc: KdlDocument = kdl_string.parse().expect("KDL parse failed");
        let parsed_list = List::try_from(parsed_doc)
            .map_err(|e| {
                for err in &e.errors {
                    eprintln!("{:?}", err);
                }
                e
            })
            .expect("KDL deserialization failed");

        // Compare the original and parsed List
        // Since SlotMap keys are not preserved, compare the Output fields and Mode values
        let orig_output = list.outputs.values().next().unwrap();
        let parsed_output = parsed_list.outputs.values().next().unwrap();

        assert_eq!(orig_output.serial_number, parsed_output.serial_number);
        assert_eq!(orig_output.name, parsed_output.name);
        assert_eq!(orig_output.enabled, parsed_output.enabled);
        assert_eq!(orig_output.mirroring, parsed_output.mirroring);
        assert_eq!(orig_output.make, parsed_output.make);
        assert_eq!(orig_output.model, parsed_output.model);
        assert_eq!(orig_output.physical, parsed_output.physical);
        assert_eq!(orig_output.position, parsed_output.position);
        assert_eq!(orig_output.scale, parsed_output.scale);
        assert_eq!(orig_output.transform, parsed_output.transform);
        assert_eq!(orig_output.adaptive_sync, parsed_output.adaptive_sync);
        assert_eq!(
            orig_output.adaptive_sync_availability,
            parsed_output.adaptive_sync_availability
        );
        assert_eq!(orig_output.xwayland_primary, parsed_output.xwayland_primary);

        // Compare modes by value (order should be preserved)
        let orig_modes: Vec<_> = orig_output.modes.iter().map(|k| &list.modes[*k]).collect();
        let parsed_modes: Vec<_> = parsed_output
            .modes
            .iter()
            .map(|k| &parsed_list.modes[*k])
            .collect();
        assert_eq!(orig_modes.len(), parsed_modes.len());
        for (a, b) in orig_modes.iter().zip(parsed_modes.iter()) {
            assert_eq!(a.size, b.size);
            assert_eq!(a.refresh_rate, b.refresh_rate);
            assert_eq!(a.preferred, b.preferred);
        }
    }
}
