# Starlight PC

![Downloads](https://img.shields.io/github/downloads/All-Of-Us-Mods/Starlight-PC/total?label=Downloads)

## Download

Get the latest release here:

https://github.com/All-Of-Us-Mods/Starlight-PC/releases/latest

## Screenshots

### Explore

<img src="screenshots/explore.png" alt="Explore Screenshot" width="800" />

### Profile

<img src="screenshots/profile.png" alt="Profile Screenshot" width="800" />

---

## Development Prerequisites

- Install Rust: https://www.rust-lang.org/tools/install
- Or use the provided Nix flake: `nix develop`

## Development

```bash
cargo run            # Start in development mode
cargo build --release # Build for production
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test  # What CI runs
```

## Tech Stack

- **UI**: [GPUI](https://www.gpui.rs/) (the Rust UI framework from [Zed](https://zed.dev/))
- **Components**: [gpui-component](https://github.com/longbridge/gpui-component)
- **HTTP**: [reqwest](https://github.com/seanmonstar/reqwest) (with rustls)

## Disclaimer

This mod launcher is not affiliated with Among Us or Innersloth LLC, and the content contained therein is not endorsed or otherwise sponsored by Innersloth LLC. Portions of the materials contained herein are property of Innersloth LLC. © Innersloth LLC.

## License

Licensed under [GPLv3](LICENSE)
