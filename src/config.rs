//! 配置模块：负责从 TOML/JSON 文件加载应用配置并做基础校验。

use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 配置相关错误类型，统一封装 IO、解析与语义错误。
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("invalid configuration: {0}")]
    Invalid(String),
}

/// 单个对端节点的连接配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConfig {
    pub host: String,
    pub port: u16,
}

/// 应用整体配置：监听端口、共享密钥、大小限制与对端列表等。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub listen_port: u16,
    pub secret_key: String,
    #[serde(default = "AppConfig::default_max_file_size")]
    pub max_file_size: u64,
    #[serde(default)]
    pub peers: Vec<PeerConfig>,
}

impl AppConfig {
    /// 默认允许的最大文件大小（10 MiB）。
    pub fn default_max_file_size() -> u64 {
        10 * 1024 * 1024
    }

    /// 推导不同平台下的默认配置文件路径。
    pub fn default_path() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
                return PathBuf::from(dir)
                    .join("lan-clipboard-sync")
                    .join("config.toml");
            }
            if let Some(home) = std::env::var_os("HOME") {
                return PathBuf::from(home)
                    .join(".config")
                    .join("lan-clipboard-sync")
                    .join("config.toml");
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(appdata) = std::env::var_os("APPDATA") {
                return PathBuf::from(appdata)
                    .join("lan-clipboard-sync")
                    .join("config.toml");
            }
        }

        PathBuf::from("lan-clipboard-sync.toml")
    }

    /// 从给定路径加载配置文件，并根据扩展名选择 TOML/JSON 解析。
    pub fn load(path: PathBuf) -> Result<Self, ConfigError> {
        let data = fs::read_to_string(&path)?;
        let cfg = if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            match ext {
                "json" => serde_json::from_str::<AppConfig>(&data)
                    .map_err(|e| ConfigError::Parse(e.to_string()))?,
                "toml" => toml::from_str::<AppConfig>(&data)
                    .map_err(|e| ConfigError::Parse(e.to_string()))?,
                _ => toml::from_str::<AppConfig>(&data)
                    .map_err(|e| ConfigError::Parse(e.to_string()))?,
            }
        } else {
            toml::from_str::<AppConfig>(&data).map_err(|e| ConfigError::Parse(e.to_string()))?
        };
        cfg.validate()?;
        Ok(cfg)
    }

    /// 对关键字段做基础校验，尽早发现明显错误。
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.listen_port == 0 {
            return Err(ConfigError::Invalid("listen_port must be > 0".into()));
        }
        let key_bytes = hex::decode(&self.secret_key)
            .map_err(|_| ConfigError::Invalid("secret_key must be valid hex string".into()))?;
        if key_bytes.len() != 32 {
            return Err(ConfigError::Invalid(
                "secret_key must be exactly 32 bytes (64 hex chars)".into(),
            ));
        }
        Ok(())
    }

    /// 将配置保存到指定路径（TOML 格式）。
    pub fn save(&self, path: &PathBuf) -> Result<(), ConfigError> {
        self.validate()?;
        let parent = path
            .parent()
            .ok_or_else(|| ConfigError::Invalid("config path has no parent directory".into()))?;
        fs::create_dir_all(parent)?;
        let toml = toml::to_string_pretty(self).map_err(|e| ConfigError::Parse(e.to_string()))?;
        fs::write(path, toml)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_ok() {
        let toml = r#"
listen_port = 5000
secret_key = "0123456789abcdef0123456789abcdef"
max_file_size = 1024

[[peers]]
host = "127.0.0.1"
port = 5001
"#;
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.listen_port, 5000);
        assert_eq!(cfg.peers.len(), 1);
    }
}
