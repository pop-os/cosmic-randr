// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use clap::{Parser, ValueEnum};
use cosmic_randr::{output_head::OutputHead, AdaptiveSyncState, Context};
use nu_ansi_term::{Color, Style};
use std::fmt::{Display, Write as FmtWrite};
use std::io::Write;
use wayland_client::protocol::wl_output::Transform as WlTransform;
use wayland_client::Proxy;

/// Display and configure wayland outputs
#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Args, Debug)]
struct Mode {
    /// Name of the output that the display is connected to.
    output: String,
    /// Specifies the height of the output picture.
    width: i32,
    /// Specifies the width of the output picture.
    height: i32,
    /// Specifies the refresh rate to apply to the output.
    #[arg(long)]
    refresh: Option<f32>,
    /// Position the output within this x pixel coordinate.
    #[arg(long)]
    pos_x: Option<i32>,
    /// Position the output within this y pixel coordinate.
    #[arg(long)]
    pos_y: Option<i32>,
    /// Changes the dimensions of the output picture.
    #[arg(long)]
    scale: Option<f32>,
    /// Tests the output configuration without applying it.
    #[arg(long)]
    test: bool,
    /// Specifies a transformation matrix to apply to the output.
    #[arg(long, value_enum)]
    transform: Option<Transform>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Disable a display
    Disable { output: String },

    /// Enable a display
    Enable { output: String },

    /// List available output heads and modes.
    List {
        /// Display in KDL format.
        #[arg(long)]
        kdl: bool,
    },

    /// Set a mode for a display.
    Mode(Mode),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, ValueEnum)]
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
            Transform::Rotate90 => "rotate-90",
            Transform::Rotate180 => "rotate-180",
            Transform::Rotate270 => "rotate-270",
            Transform::Flipped => "flipped",
            Transform::Flipped90 => "flipped-90",
            Transform::Flipped180 => "flipped-180",
            Transform::Flipped270 => "flipped-270",
        })
    }
}

impl TryFrom<WlTransform> for Transform {
    type Error = &'static str;

    fn try_from(transform: WlTransform) -> Result<Self, Self::Error> {
        Ok(match transform {
            WlTransform::Normal => Transform::Normal,
            WlTransform::_90 => Transform::Rotate90,
            WlTransform::_180 => Transform::Rotate180,
            WlTransform::_270 => Transform::Rotate270,
            WlTransform::Flipped => Transform::Flipped,
            WlTransform::Flipped90 => Transform::Flipped90,
            WlTransform::Flipped180 => Transform::Flipped180,
            WlTransform::Flipped270 => Transform::Flipped270,
            _ => return Err("unknown wl_transform variant"),
        })
    }
}

impl Transform {
    #[must_use]
    pub fn wl_transform(self) -> WlTransform {
        match self {
            Transform::Normal => WlTransform::Normal,
            Transform::Rotate90 => WlTransform::_90,
            Transform::Rotate180 => WlTransform::_180,
            Transform::Rotate270 => WlTransform::_270,
            Transform::Flipped => WlTransform::Flipped,
            Transform::Flipped90 => WlTransform::Flipped90,
            Transform::Flipped180 => WlTransform::Flipped180,
            Transform::Flipped270 => WlTransform::Flipped270,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let (message_tx, mut message_rx) = tachyonix::channel(4);

    let (mut context, mut event_queue) = cosmic_randr::connect(message_tx)?;

    let _res = event_queue.roundtrip(&mut context);

    match cli.command {
        Commands::Enable { output } => loop {
            tokio::select! {
                _res = context.dispatch(&mut event_queue) => {
                    return enable(&mut context, &output);
                }

                message = message_rx.recv() => {
                    if config_message(message)? {
                        return Ok(());
                    }
                }
            }
        },

        Commands::Disable { output } => loop {
            tokio::select! {
                _res = context.dispatch(&mut event_queue) => {
                    return disable(&mut context, &output);
                }

                message = message_rx.recv() => {
                    if config_message(message)? {
                        return Ok(());
                    }
                }
            }
        },

        Commands::List { kdl } => {
            let _res = context.dispatch(&mut event_queue).await;

            if kdl {
                list_kdl(&context);
            } else {
                list(&context);
            }

            return Ok(());
        }

        Commands::Mode(mode) => loop {
            tokio::select! {
                _res = context.dispatch(&mut event_queue) => {
                    set_mode(&mut context, &mode)?;
                }

                message = message_rx.recv() => {
                    if config_message(message)? {
                        return Ok(());
                    }
                }
            }
        },
    }
}

/// Handles output configuration messages.
///
/// # Errors
///
/// - Error if the output configuration returned an error.
/// - Or if the channel is disconnected.
pub fn config_message(
    message: Result<cosmic_randr::Message, tachyonix::RecvError>,
) -> Result<bool, Box<dyn std::error::Error>> {
    match message {
        Ok(cosmic_randr::Message::ConfigurationCancelled) => Err("configuration cancelled".into()),

        Ok(cosmic_randr::Message::ConfigurationFailed) => Err("configuration failed".into()),

        Ok(cosmic_randr::Message::ConfigurationSucceeded) => Ok(true),

        Err(why) => Err(format!("channel error: {why:?}").into()),

        _ => Ok(false),
    }
}

fn get_output_head(
    context: &mut Context,
    output: &str,
) -> Result<OutputHead, Box<dyn std::error::Error>> {
    context
        .output_heads
        .iter()
        .filter(|(_id, head)| head.wlr_head.is_alive())
        .find(|(_id, head)| head.name == output)
        .map(|(_id, head)| (head.clone()))
        .ok_or_else(|| "could not find display".into())
}

fn disable(context: &mut Context, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let head = get_output_head(context, output)?;

    let config = context.create_output_config();
    config.disable_head(&head.wlr_head);
    config.apply();

    Ok(())
}

fn enable(context: &mut Context, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let head = get_output_head(context, output)?;

    let config = context.create_output_config();
    config.enable_head(&head.wlr_head, &context.handle, context.data);
    config.apply();

    Ok(())
}

fn list(context: &Context) {
    let mut output = String::new();
    let mut resolution = String::new();

    for head in context.output_heads.values() {
        #[allow(clippy::ignored_unit_patterns)]
        let _res = fomat_macros::witeln!(
            &mut output,
            (Style::new().bold().paint(&head.name)) " "
            if head.enabled {
                (Color::Green.bold().paint("(enabled)"))
            } else {
                (Color::Red.bold().paint("(disabled)"))
            }
            if !head.make.is_empty() {
                (Color::Yellow.bold().paint("\n  Make: ")) (head.make)
            }
            (Color::Yellow.bold().paint("\n  Model: "))
            (head.model)
            (Color::Yellow.bold().paint("\n  Physical Size: "))
            (head.physical_width) " x " (head.physical_height) " mm"
            (Color::Yellow.bold().paint("\n  Position: "))
            (head.position_x) "," (head.position_y)
            if let Some(wl_transform) = head.transform {
                if let Ok(transform) = Transform::try_from(wl_transform) {
                    (Color::Yellow.bold().paint("\n  Transform: ")) (transform)
                }
            }
            if let Some(sync) = head.adaptive_sync {
                (Color::Yellow.bold().paint("\n  Adaptive Sync: "))
                if let AdaptiveSyncState::Enabled = sync {
                    (Color::Green.paint("true\n"))
                } else {
                    (Color::Red.paint("false\n"))
                }
            }
            (Color::Yellow.bold().paint("\n  Modes:"))
        );

        for mode_id in &head.modes {
            let Some(mode) = context.output_modes.get(mode_id) else {
                continue;
            };

            resolution.clear();
            let _res = write!(&mut resolution, "{}x{}", mode.width, mode.height);

            let _res = writeln!(
                &mut output,
                "    {:>9} @ {}{}{}",
                Color::Magenta.paint(format!("{resolution:>9}")),
                Color::Cyan.paint(format!(
                    "{:>3}.{:02} Hz",
                    mode.refresh / 1000,
                    mode.refresh % 1000
                )),
                if head.current_mode.as_ref() == Some(mode_id) {
                    Color::Purple.bold().paint(" (current)")
                } else {
                    Color::default().paint("")
                },
                if mode.preferred {
                    Color::Green.bold().paint(" (preferred)")
                } else {
                    Color::default().paint("")
                }
            );
        }
    }

    let mut stdout = std::io::stdout().lock();
    let _res = stdout.write_all(output.as_bytes());
    let _res = stdout.flush();
}

fn list_kdl(context: &Context) {
    let mut output = String::new();

    for head in context.output_heads.values() {
        #[allow(clippy::ignored_unit_patterns)]
        let _res = fomat_macros::witeln!(
            &mut output,
            "output \"" (head.name) "\" enabled=" (head.enabled) " {\n"
            "  description"
            if !head.make.is_empty() { " make=\"" (head.make) "\"" }
            " model=\"" (head.model) "\"\n"
            "  physical " (head.physical_width) " " (head.physical_height) "\n"
            "  position " (head.position_x) " " (head.position_y) "\n"
            if let Some(wl_transform) = head.transform {
                if let Ok(transform) = Transform::try_from(wl_transform) {
                    "  transform \"" (transform) "\"\n"
                }
            }
            if let Some(sync) = head.adaptive_sync {
                "  adaptive_sync "
                if let AdaptiveSyncState::Enabled = sync {
                    "true\n"
                } else {
                    "false\n"
                }
            }
            if !head.serial_number.is_empty() {
                "  serial_number=\"" (head.serial_number) "\"\n"
            }
            "  modes {"
        );

        for mode_id in &head.modes {
            let Some(mode) = context.output_modes.get(mode_id) else {
                continue;
            };

            let _res = writeln!(
                &mut output,
                "    mode {} {} {}{}{}",
                mode.width,
                mode.height,
                mode.refresh,
                if head.current_mode.as_ref() == Some(mode_id) {
                    " current=true"
                } else {
                    ""
                },
                if mode.preferred {
                    " preferred=true"
                } else {
                    ""
                },
            );
        }

        let _res = writeln!(&mut output, "  }}\n}}");
    }

    let mut stdout = std::io::stdout().lock();
    let _res = stdout.write_all(output.as_bytes());
    let _res = stdout.flush();
}

fn set_mode(context: &mut Context, args: &Mode) -> Result<(), Box<dyn std::error::Error>> {
    let head = get_output_head(context, &args.output)?;

    let config = context.create_output_config();
    let head_config = config.enable_head(&head.wlr_head, &context.handle, context.data);

    if let Some((x, y)) = args.pos_x.zip(args.pos_y) {
        head_config.set_position(x, y);
    }

    if let Some(scale) = args.scale {
        head_config.set_scale(f64::from(scale));
    }

    let mode_iter = || {
        head.modes
            .iter()
            .filter_map(|mode_id| context.output_modes.get(mode_id))
            .filter(|mode| mode.width == args.width && mode.height == args.height)
    };

    if let Some(transform) = args.transform {
        head_config.set_transform(transform.wl_transform());
    }

    if let Some(refresh) = args.refresh {
        #[allow(clippy::cast_possible_truncation)]
        let refresh = (refresh * 1000.0) as i32;

        let min = refresh - 501;
        let max = refresh + 501;

        if let Some(mode) = mode_iter().find(|mode| min < mode.refresh && max > mode.refresh) {
            head_config.set_mode(&mode.wlr_mode);

            if args.test {
                config.test();
            } else {
                config.apply();
            }

            return Ok(());
        }
    }

    let Some(mode) = mode_iter().next() else {
        return Err("could not find matching mode for display".into());
    };

    head_config.set_mode(&mode.wlr_mode);

    if args.test {
        config.test();
    } else {
        config.apply();
    }

    Ok(())
}
