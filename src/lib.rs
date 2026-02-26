mod clipboard;
mod config;
mod core;
mod crypto;
mod network;
pub mod protocol;
#[cfg(any(target_os = "linux", target_os = "windows"))]
mod tray;

pub use clipboard::{ClipboardFile, ClipboardItem};
pub use config::{AppConfig, PeerConfig};
pub use core::CoreService;
#[cfg(any(target_os = "linux", target_os = "windows"))]
pub use tray::{TrayEvent, TrayManager};
