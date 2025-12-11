// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

pub mod align;

use clap::{Parser, ValueEnum};
use cosmic_randr::Message;
use cosmic_randr::context::HeadConfiguration;
use cosmic_randr::{AdaptiveSyncAvailability, AdaptiveSyncStateExt, Context};
use cosmic_randr_shell::{KdlParseWithError, List};
use nu_ansi_term::{Color, Style};
use std::fmt::{Display, Write as FmtWrite};
use std::io::Write;
use wayland_client::protocol::wl_output::Transform as WlTransform;
use wayland_client::{EventQueue, Proxy};

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
    /// Specifies the width of the output picture.
    width: i32,
    /// Specifies the height of the output picture.
    height: i32,
    /// Specifies the refresh rate to apply to the output.
    #[arg(long)]
    refresh: Option<f32>,
    /// Specfies the adaptive sync mode to apply to the output.
    #[arg(long, value_enum)]
    adaptive_sync: Option<AdaptiveSync>,
    /// Position the output within this x pixel coordinate.
    #[arg(long, allow_hyphen_values(true))]
    pos_x: Option<i32>,
    /// Position the output within this y pixel coordinate.
    #[arg(long, allow_hyphen_values(true))]
    pos_y: Option<i32>,
    /// Changes the dimensions of the output picture.
    #[arg(long)]
    scale: Option<f64>,
    /// Tests the output configuration without applying it.
    #[arg(long)]
    test: bool,
    /// Specifies a transformation matrix to apply to the output.
    #[arg(long, value_enum)]
    transform: Option<Transform>,
}

impl Mode {
    fn to_head_config(&self) -> HeadConfiguration {
        HeadConfiguration {
            size: Some((self.width as u32, self.height as u32)),
            refresh: self.refresh,
            adaptive_sync: self
                .adaptive_sync
                .map(|adaptive_sync| adaptive_sync.adaptive_sync_state_ext()),
            pos: (self.pos_x.is_some() || self.pos_y.is_some()).then(|| {
                (
                    self.pos_x.unwrap_or_default(),
                    self.pos_y.unwrap_or_default(),
                )
            }),
            scale: self.scale,
            transform: self.transform.map(|transform| transform.wl_transform()),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Disable a display
    Disable { output: String },

    /// Enable a display
    Enable { output: String },

    /// Mirror a display
    Mirror { output: String, from: String },

    /// List available output heads and modes.
    List {
        /// Display in KDL format.
        #[arg(long)]
        kdl: bool,
    },

    /// Set a mode for a display.
    Mode(Mode),

    /// Set position of display.
    Position {
        output: String,
        x: i32,
        y: i32,
        #[arg(long)]
        test: bool,
    },

    /// Xwayland compatibility options
    #[command(arg_required_else_help = true)]
    Xwayland {
        /// Set output as primary
        #[arg(long, value_name = "OUTPUT")]
        primary: Option<String>,
        /// Unset primary output
        #[arg(long, conflicts_with = "primary")]
        no_primary: bool,
    },

    /// List of output configurations to apply in KDL format
    /// Read via stdin
    Kdl,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum AdaptiveSync {
    #[value(name = "true")]
    Always,
    #[value(name = "automatic")]
    Automatic,
    #[value(name = "false")]
    Disabled,
}

impl Display for AdaptiveSync {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AdaptiveSync::Always => "true",
            AdaptiveSync::Automatic => "automatic",
            AdaptiveSync::Disabled => "false",
        })
    }
}

impl TryFrom<AdaptiveSyncStateExt> for AdaptiveSync {
    type Error = &'static str;

    fn try_from(value: AdaptiveSyncStateExt) -> Result<Self, Self::Error> {
        Ok(match value {
            AdaptiveSyncStateExt::Always => AdaptiveSync::Always,
            AdaptiveSyncStateExt::Automatic => AdaptiveSync::Automatic,
            AdaptiveSyncStateExt::Disabled => AdaptiveSync::Disabled,
            _ => return Err("unknown adaptive_sync_state_ext variant"),
        })
    }
}

impl AdaptiveSync {
    #[must_use]
    pub fn adaptive_sync_state_ext(self) -> AdaptiveSyncStateExt {
        match self {
            AdaptiveSync::Always => AdaptiveSyncStateExt::Always,
            AdaptiveSync::Automatic => AdaptiveSyncStateExt::Automatic,
            AdaptiveSync::Disabled => AdaptiveSyncStateExt::Disabled,
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let (message_tx, message_rx) = cosmic_randr::channel();

    let (context, event_queue) = cosmic_randr::connect(message_tx)?;

    let mut app = App {
        context,
        event_queue,
        message_rx,
    };

    match cli.command {
        Commands::Enable { output } => app.enable(&output).await,

        Commands::Mirror { output, from } => app.mirror(&output, &from).await,

        Commands::Disable { output } => app.disable(&output).await,

        Commands::List { kdl } => app.list(kdl).await,

        Commands::Mode(mode) => app.mode(mode).await,

        Commands::Position { output, x, y, test } => app.set_position(&output, x, y, test).await,

        Commands::Xwayland { primary, .. } => app.set_xwayland_primary(primary.as_deref()).await,

        Commands::Kdl => {
            let mut input = String::new();
            use tokio::io::AsyncReadExt;
            tokio::io::stdin()
                .read_to_string(&mut input)
                .await
                .expect("Failed to read stdin");
            let doc = kdl::KdlDocument::parse(&input).expect("Invalid KDL");

            let list: List = match cosmic_randr_shell::List::try_from(doc) {
                Ok(l) => l,
                Err(KdlParseWithError { list, errors }) => {
                    eprintln!("{errors:?}");
                    list
                }
            };
            app.apply_list(list).await
        }
    }
}

struct App {
    context: Context,
    event_queue: EventQueue<Context>,
    message_rx: cosmic_randr::Receiver,
}

impl App {
    // Ignores any messages other than `ManagerDone`
    async fn dispatch_until_manager_done(&mut self) -> Result<(), cosmic_randr::Error> {
        loop {
            let watcher = async {
                while let Some(msg) = self.message_rx.recv().await {
                    if matches!(msg, Message::ManagerDone) {
                        return true;
                    }
                }

                false
            };

            tokio::select! {
                is_done = watcher => {
                    if is_done {
                        break
                    }
                },

                result = self.context.dispatch(&mut self.event_queue) => {
                    result?;
                }
            };
        }

        Ok(())
    }

    /// # Errors
    ///
    /// Returns error if the message receiver fails, dispach fails, or a configuration failed.
    async fn receive_config_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            while let Some(message) = self.message_rx.try_recv() {
                if config_message(Some(message))? {
                    return Ok(());
                }
            }

            self.context.dispatch(&mut self.event_queue).await?;
        }
    }

    async fn enable(&mut self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        enable(&mut self.context, output)?;
        self.receive_config_messages().await?;

        Ok(())
    }

    async fn mirror(&mut self, output: &str, from: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        mirror(&mut self.context, output, from)?;
        self.receive_config_messages().await
    }

    async fn disable(&mut self, output: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        disable(&mut self.context, output)?;
        self.receive_config_messages().await
    }

    async fn list(&mut self, kdl: bool) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        for head in self.context.output_heads.values_mut() {
            head.modes
                .sort_unstable_by(|_, either, _, or| either.cmp(or));
        }

        if kdl {
            list_kdl(&self.context);
        } else {
            list(&self.context);
        }

        Ok(())
    }

    async fn mode(&mut self, mode: Mode) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        set_mode(&mut self.context, &mode)?;
        self.receive_config_messages().await?;
        self.auto_correct_offsets(&mode.output, mode.test).await
    }

    async fn set_position(
        &mut self,
        output: &str,
        x: i32,
        y: i32,
        test: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        set_position(&mut self.context, output, x, y, test)?;
        self.receive_config_messages().await?;
        self.auto_correct_offsets(output, test).await
    }

    async fn set_xwayland_primary(
        &mut self,
        output: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;
        self.context.set_xwayland_primary(output)?;
        self.dispatch_until_manager_done().await?;
        Ok(())
    }

    // Offset outputs in case of negative positioning.
    async fn auto_correct_offsets(
        &mut self,
        output: &str,
        test: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get the position and dimensions of the moved display.
        let Some(ref mut active_output) = self
            .context
            .output_heads
            .values()
            .find(|head| head.name == output)
            .and_then(|head| {
                let mode = head.current_mode.as_ref()?;
                let mode = head.modes.get(mode)?;

                let (width, height) = if head.transform.is_none_or(|wl_transform| {
                    Transform::try_from(wl_transform).map_or(true, is_landscape)
                }) {
                    (mode.width, mode.height)
                } else {
                    (mode.height, mode.width)
                };

                Some(align::Rectangle {
                    x: head.position_x as f32,
                    y: head.position_y as f32,
                    width: width as f32 / head.scale as f32,
                    height: height as f32 / head.scale as f32,
                })
            })
        else {
            return Ok(());
        };

        // Create an iterator of other outputs and their positions and dimensions.
        let other_outputs = self.context.output_heads.values().filter_map(|head| {
            if head.name == output {
                None
            } else {
                let mode = head.current_mode.as_ref()?;
                let mode = head.modes.get(mode)?;

                if !head.enabled || head.mirroring.is_some() {
                    return None;
                }

                let (width, height) = if head.transform.is_none_or(|wl_transform| {
                    Transform::try_from(wl_transform).map_or(true, is_landscape)
                }) {
                    (mode.width, mode.height)
                } else {
                    (mode.height, mode.width)
                };

                Some(align::Rectangle {
                    x: head.position_x as f32,
                    y: head.position_y as f32,
                    width: width as f32 / head.scale as f32,
                    height: height as f32 / head.scale as f32,
                })
            }
        });

        // Align outputs such that there are no gaps.
        align::display(active_output, other_outputs);

        // Calculate how much to offset the position of each display to be aligned against (0,0)
        let mut offset = self
            .context
            .output_heads
            .values()
            .filter(|head| head.enabled && head.mirroring.is_none())
            .fold((i32::MAX, i32::MAX), |offset, head| {
                let (x, y) = if output == head.name {
                    (active_output.x as i32, active_output.y as i32)
                } else {
                    (head.position_x, head.position_y)
                };

                (offset.0.min(x), offset.1.min(y))
            });

        // Reposition each display with that offset
        let updates = self
            .context
            .output_heads
            .values()
            .filter(|head| head.enabled && head.mirroring.is_none())
            .map(|head| {
                let (x, y) = if output == head.name {
                    (active_output.x as i32, active_output.y as i32)
                } else {
                    (head.position_x, head.position_y)
                };

                (head.name.clone(), x - offset.0, y - offset.1)
            })
            .collect::<Vec<_>>();

        // Adjust again to (0,0) baseline
        offset = updates
            .iter()
            .fold((i32::MAX, i32::MAX), |offset, (_, x, y)| {
                (offset.0.min(*x), offset.1.min(*y))
            });

        // Apply new positions
        for (name, mut x, mut y) in updates {
            x -= offset.0;
            y -= offset.1;
            set_position(&mut self.context, &name, x, y, test)?;
            self.receive_config_messages().await?;
        }

        Ok(())
    }

    /// Apply requested output configuration all at once using the protocol
    async fn apply_list(&mut self, mut list: List) -> Result<(), Box<dyn std::error::Error>> {
        self.dispatch_until_manager_done().await?;

        // convert list to hashmap of output heads

        let mut current_heads: Vec<_> = self.context.output_heads.values_mut().collect();

        for (_, head) in list.outputs.drain() {
            for current in &mut current_heads {
                if current.name == head.name
                    && current.make == head.clone().make.unwrap_or_default()
                    && current.model == head.model
                {
                    current.adaptive_sync = head.adaptive_sync.map(|sync| match sync {
                        cosmic_randr_shell::AdaptiveSyncState::Always => {
                            AdaptiveSyncStateExt::Always
                        }
                        cosmic_randr_shell::AdaptiveSyncState::Auto => {
                            AdaptiveSyncStateExt::Automatic
                        }
                        cosmic_randr_shell::AdaptiveSyncState::Disabled => {
                            AdaptiveSyncStateExt::Disabled
                        }
                    });
                    current.enabled = head.enabled;
                    current.position_x = head.position.0;
                    current.position_y = head.position.1;
                    current.scale = head.scale;
                    current.transform = head.transform.map(|t| match t {
                        cosmic_randr_shell::Transform::Normal => WlTransform::Normal,
                        cosmic_randr_shell::Transform::Rotate90 => WlTransform::_90,
                        cosmic_randr_shell::Transform::Rotate180 => WlTransform::_180,
                        cosmic_randr_shell::Transform::Rotate270 => WlTransform::_270,
                        cosmic_randr_shell::Transform::Flipped => WlTransform::Flipped,
                        cosmic_randr_shell::Transform::Flipped90 => WlTransform::Flipped90,
                        cosmic_randr_shell::Transform::Flipped180 => WlTransform::Flipped180,
                        cosmic_randr_shell::Transform::Flipped270 => WlTransform::Flipped270,
                    });
                    current.mirroring = head.mirroring.clone();
                    current.xwayland_primary = head.xwayland_primary;
                    if let Some(cur_mode_id) = head
                        .current
                        .and_then(|k| list.modes.get(k))
                        .and_then(|mode_info| {
                            current.modes.iter_mut().find_map(|(id, mode)| {
                                if mode.width == mode_info.size.0 as i32
                                    && mode.height == mode_info.size.1 as i32
                                {
                                    mode.refresh = mode_info.refresh_rate as i32;
                                    mode.preferred = mode_info.preferred;
                                    Some(id.clone())
                                } else {
                                    None
                                }
                            })
                        })
                    {
                        current.current_mode = Some(cur_mode_id);
                    }

                    break;
                }
            }
        }

        self.context.apply_current_config().await?;
        self.receive_config_messages().await
    }
}

/// Handles output configuration messages.
///
/// # Errors
///
/// - Error if the output configuration returned an error.
/// - Or if the channel is disconnected.
pub fn config_message(
    message: Option<cosmic_randr::Message>,
) -> Result<bool, Box<dyn std::error::Error>> {
    match message {
        Some(cosmic_randr::Message::ConfigurationCancelled) => {
            Err("configuration cancelled".into())
        }

        Some(cosmic_randr::Message::ConfigurationFailed) => Err("configuration failed".into()),

        Some(cosmic_randr::Message::ConfigurationSucceeded) => Ok(true),
        _ => Ok(false),
    }
}

fn disable(context: &mut Context, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = context.create_output_config();
    config.disable_head(output)?;
    config.apply();

    Ok(())
}

fn enable(context: &mut Context, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = context.create_output_config();
    config.enable_head(output, None)?;
    config.apply();

    Ok(())
}

fn mirror(
    context: &mut Context,
    output: &str,
    from: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = context.create_output_config();
    config.mirror_head(output, from, None)?;
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
                if let Some(from) = head.mirroring.as_ref() {
                    (Color::Blue.bold().paint(format!("(mirroring \"{}\")", from)))
                } else {
                    (Color::Green.bold().paint("(enabled)"))
                }
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
            (Color::Yellow.bold().paint("\n  Scale: ")) ((head.scale * 100.0) as i32) "%"
            if let Some(wl_transform) = head.transform {
                if let Ok(transform) = Transform::try_from(wl_transform) {
                    (Color::Yellow.bold().paint("\n  Transform: ")) (transform)
                }
            }
            if let Some(available) = head.adaptive_sync_support {
                (Color::Yellow.bold().paint("\n  Adaptive Sync Support: "))
                (match available {
                    AdaptiveSyncAvailability::Supported | AdaptiveSyncAvailability::RequiresModeset => Color::Green.paint("true"),
                    _ => Color::Red.paint("false"),
                })
            }
            if let Some(sync) = head.adaptive_sync {
                (Color::Yellow.bold().paint("\n  Adaptive Sync: "))
                (match sync {
                    AdaptiveSyncStateExt::Always => {
                        Color::Green.paint("true")
                    },
                    AdaptiveSyncStateExt::Automatic => {
                        Color::Green.paint("automatic")
                    },
                    _ => {
                        Color::Red.paint("false")
                    }
                })
            }
            if let Some(xwayland_primary) = head.xwayland_primary {
                (Color::Yellow.bold().paint("\n  Xwayland primary: "))
                (if xwayland_primary {
                    Color::Green.paint("true")
                } else {
                    Color::Red.paint("false")
                })
            }
            (Color::Yellow.bold().paint("\n\n  Modes:"))
        );

        for mode in head.modes.values() {
            resolution.clear();
            let _res = write!(&mut resolution, "{}x{}", mode.width, mode.height);

            let _res = writeln!(
                &mut output,
                "    {:>9} @ {}{}{}",
                Color::Magenta.paint(format!("{resolution:>9}")),
                Color::Cyan.paint(format!(
                    "{:>3}.{:03} Hz",
                    mode.refresh / 1000,
                    mode.refresh % 1000
                )),
                if head.current_mode.as_ref() == Some(&mode.wlr_mode.id()) {
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
            "output \"" (head.name) "\" enabled=#" (head.enabled) " {\n"
            "  description"
            if !head.make.is_empty() { " make=\"" (head.make) "\"" }
            " model=\"" (head.model) "\"\n"
            "  physical " (head.physical_width) " " (head.physical_height) "\n"
            "  position " (head.position_x) " " (head.position_y) "\n"
            "  scale " (format!("{:.2}", head.scale)) "\n"
            if let Some(mirroring) = head.mirroring.as_ref() {
                "  mirroring \"" (mirroring) "\"\n"
            }
            if let Some(wl_transform) = head.transform {
                if let Ok(transform) = Transform::try_from(wl_transform) {
                    "  transform \"" (transform) "\"\n"
                }
            }
            if let Some(available) = head.adaptive_sync_support {
                "  adaptive_sync_support "
                (match available {
                    AdaptiveSyncAvailability::Supported => "#true",
                    AdaptiveSyncAvailability::RequiresModeset => "\"requires_modeset\"",
                    _ => "#false",
                })
                "\n"
            }
            if let Some(sync) = head.adaptive_sync {
                "  adaptive_sync "
                (match sync {
                    AdaptiveSyncStateExt::Always => "#true",
                    AdaptiveSyncStateExt::Automatic => "\"automatic\"",
                    _ => "#false",
                })
                "\n"
            }
            if let Some(xwayland_primary) = head.xwayland_primary {
                "  xwayland_primary "
                (if xwayland_primary {
                    "#true"
                } else {
                    "#false"
                })
                "\n"
            }
            if !head.serial_number.is_empty() {
                "  serial_number \"" (head.serial_number) "\"\n"
            }
            "  modes {"
        );

        for mode in head.modes.values() {
            let _res = writeln!(
                &mut output,
                "    mode {} {} {}{}{}",
                mode.width,
                mode.height,
                mode.refresh,
                if head.current_mode.as_ref() == Some(&mode.wlr_mode.id()) {
                    " current=#true"
                } else {
                    ""
                },
                if mode.preferred {
                    " preferred=#true"
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
    let mirroring = context
        .output_heads
        .values()
        .find(|output| output.name == args.output)
        .and_then(|head| head.mirroring.clone());

    let mut config = context.create_output_config();
    let head_config = args.to_head_config();

    if let Some(mirroring_from) = mirroring.filter(|_| head_config.pos.is_none()) {
        config.mirror_head(&args.output, &mirroring_from, Some(head_config))?;
    } else {
        config.enable_head(&args.output, Some(head_config))?;
    }

    if args.test {
        config.test();
    } else {
        config.apply();
    }

    Ok(())
}

fn set_position(
    context: &mut Context,
    name: &str,
    x: i32,
    y: i32,
    test: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = context.create_output_config();
    config.enable_head(
        name,
        Some(HeadConfiguration {
            pos: Some((x, y)),
            ..Default::default()
        }),
    )?;

    if test {
        config.test();
    } else {
        config.apply();
    }

    Ok(())
}

fn is_landscape(transform: Transform) -> bool {
    matches!(
        transform,
        Transform::Normal | Transform::Rotate180 | Transform::Flipped | Transform::Flipped180
    )
}
