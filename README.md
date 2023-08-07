# Limine Rust Bare Bones

This repository adds a nix flake to the limine rust barebones template.

## How to use
Use `nix develop` to enter the development shell. Then use make as normal (you probably want `make run-uefi`).

Note: The flake uses the most recent nightly version of rust, if it doesn't work try rustc 1.73.0-nightly (f3623871c 2023-08-06).
