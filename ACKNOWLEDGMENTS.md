# Acknowledgments

This project was developed with significant assistance from **Claude** (Anthropic),
an AI coding assistant integrated into the Cursor IDE.

Claude contributed extensively to:

- **Architecture design**: overall module structure, protocol specification, and
  encryption scheme selection.
- **Implementation**: writing the core codebase across all modules — configuration
  parsing, ChaCha20-Poly1305 encryption, custom TCP framing protocol, clipboard
  abstraction layer, network broadcasting, and the main event loop with echo
  suppression logic.
- **Debugging**: diagnosing and fixing runtime issues including GTK initialization
  on Linux, Tokio runtime context errors, `file://` URI handling for clipboard
  file paths, and clipboard watcher compatibility on Wayland/KDE.
- **Testing**: unit tests for configuration, encryption, protocol encoding/decoding,
  and integration test scaffolding.
- **Documentation**: README, inline code comments, and this acknowledgments file.

The collaboration was conducted through iterative conversation — the human author
provided requirements, tested on real hardware, and reported issues; Claude proposed
solutions, wrote code, and iterated on fixes.

## Third-Party Dependencies

This project relies on the following excellent open-source crates:

| Crate | Purpose |
|---|---|
| [clipboard-rs](https://crates.io/crates/clipboard-rs) | Cross-platform clipboard read/write |
| [chacha20poly1305](https://crates.io/crates/chacha20poly1305) | Authenticated encryption (AEAD) |
| [tokio](https://crates.io/crates/tokio) | Async runtime for TCP networking |
| [serde](https://crates.io/crates/serde) / [toml](https://crates.io/crates/toml) / [serde_json](https://crates.io/crates/serde_json) | Configuration serialization |
| [tracing](https://crates.io/crates/tracing) | Structured logging |
| [anyhow](https://crates.io/crates/anyhow) / [thiserror](https://crates.io/crates/thiserror) | Error handling |

Thank you to all the maintainers and contributors of these projects.
