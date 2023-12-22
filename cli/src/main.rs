// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

use clap::Parser;
use cosmic_randr::{AdaptiveSyncState, Context};
use std::{fmt::Write as FmtWrite, io::Write};
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
    #[arg(long)]
    transform: Option<String>,
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
            }
        },

        Commands::Disable { output } => loop {
            tokio::select! {
                _res = context.dispatch(&mut event_queue) => {
                    return disable(&mut context, &output);
                }
            }
        },

        Commands::List { kdl } => loop {
            tokio::select! {
                _res = context.dispatch(&mut event_queue) => {
                    if kdl {
                        list_kdl(&context);
                    } else {
                        list(&context);
                    }
                    return Ok(());
                }
            }
        },

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

fn disable(context: &mut Context, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(head) = context
        .output_heads
        .iter()
        .filter(|(_id, head)| head.wlr_head.is_alive())
        .find(|(_id, head)| head.name == output)
        .map(|(_id, head)| (head.clone()))
    else {
        return Err("could not find display".into());
    };

    let config = context.create_output_config();

    config.disable_head(&head.wlr_head);
    config.apply();

    Ok(())
}

fn enable(context: &mut Context, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let Some(head) = context
        .output_heads
        .iter()
        .filter(|(_id, head)| head.wlr_head.is_alive())
        .find(|(_id, head)| head.name == output)
        .map(|(_id, head)| (head.clone()))
    else {
        return Err("could not find display".into());
    };

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
            (head.name) "\n"
            "  Enabled: " (head.enabled) "\n"
            "  Make: "
            if head.make.is_empty() {
                "Unknown"
            } else {
                (head.make)
            }
            "\n"
            "  Model: " (head.model) "\n"
            "  PhysicalSize: " (head.physical_width) "x" (head.physical_height) " mm\n"
            "  Position: " (head.position_x) "," (head.position_y) "\n"
            if let Some(sync) = head.adaptive_sync {
                "    Adaptive Sync: "
                if let AdaptiveSyncState::Enabled = sync {
                    "true\n"
                } else {
                    "false\n"
                }
            }
            "  Modes:"
        );

        for mode_id in &head.modes {
            let Some(mode) = context.output_modes.get(mode_id) else {
                continue;
            };

            resolution.clear();
            let _res = write!(&mut resolution, "{}x{}", mode.width, mode.height);

            let _res = writeln!(
                &mut output,
                "    {:>9} @ {:>3}.{:02} Hz{}{}",
                &resolution,
                mode.refresh / 1000,
                mode.refresh % 1000,
                if mode.preferred { " (preferred)" } else { "" },
                if head.current_mode.as_ref() == Some(mode_id) {
                    " (current)"
                } else {
                    ""
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
    let Some(head) = context
        .output_heads
        .iter()
        .filter(|(_id, head)| head.wlr_head.is_alive())
        .find(|(_id, head)| head.name == args.output)
        .map(|(_id, head)| (head.clone()))
    else {
        return Err("could not find display".into());
    };

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
