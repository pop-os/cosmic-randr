// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use kdl::{KdlDocument, KdlError};
use slotmap::{DefaultKey, SlotMap};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Mode {
    size: (u32, u32),
    refresh_rate: u32,
    preferred: bool,
}

impl Mode {
    pub const fn new() -> Self {
        Self {
            size: (0, 0),
            refresh_rate: 0,
            preferred: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Outputs {
    outputs: HashMap<String, Output>,
    modes: SlotMap<DefaultKey, Mode>,
}

#[derive(Clone, Debug)]
pub struct Output {
    enabled: bool,
    make: Option<String>,
    model: String,
    physical: (u32, u32),
    position: (u32, u32),
    modes: Vec<DefaultKey>,
    current: Option<DefaultKey>,
}

impl Output {
    pub const fn new() -> Self {
        Self {
            enabled: false,
            make: None,
            model: String::new(),
            physical: (0, 0),
            position: (0, 0),
            modes: Vec::new(),
            current: None,
        }
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
pub fn list() -> Result<Outputs, Error> {
    // Get a list of outputs from `cosmic-randr` in KDL format.
    let stdout = std::process::Command::new("cosmic-randr")
        .arg("list")
        .arg("--kdl")
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

    let mut outputs = Outputs {
        outputs: HashMap::new(),
        modes: SlotMap::new(),
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

                        match entry.name().map(|name| name.value()) {
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
                            x_pos.value().as_i64().unwrap_or_default() as u32,
                            y_pos.value().as_i64().unwrap_or_default() as u32,
                        );
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
                                match entry.name().map(|name| name.value()) {
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

                _ => (),
            }
        }

        outputs.outputs.insert(name.to_owned(), output);
    }

    Ok(outputs)
}
