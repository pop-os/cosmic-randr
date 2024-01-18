# cosmic-randr

COSMIC RandR is both a library and command line utility for displaying and configuring Wayland outputs. Each display is represented as an "output head", whereas all supported configurations for each display is represented as "output modes".

## cosmic-randr cli

All COSMIC installations have `cosmic-randr` preinstalled on the system. This can be used to list and configure outputs from the terminal.

Those that want to integrate with this binary in their software can use `cosmic-randr list --kdl` to get a list of outputs and their modes in the [KDL syntax format](https://kdl.dev). Rust developers can use the `cosmic-randr-shell` crate provided here for the same integration.

## License

Licensed under the [Mozilla Public License 2.0](https://choosealicense.com/licenses/mpl-2.0).

### Contribution

Any contribution intentionally submitted for inclusion in the work by you shall be licensed under the Mozilla Public License 2.0 (MPL-2.0). Each source file should have a SPDX copyright notice at the top of the file:

```
// SPDX-License-Identifier: MPL-2.0
```
