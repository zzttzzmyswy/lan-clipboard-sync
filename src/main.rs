use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use lan_clipboard_sync::{AppConfig, CoreService};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 指定配置文件的路径
    #[arg(short, long)]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    init_logging();

    let config_path = resolve_config_path(args.config);
    let config = AppConfig::load(config_path)?;

    let rt = tokio::runtime::Runtime::new()?;
    let mut core = CoreService::new(config)?;

    rt.block_on(async move { core.run().await })
}

fn init_logging() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn resolve_config_path(arg: Option<PathBuf>) -> PathBuf {
    // 如果通过命令行参数指定了路径，则使用该路径；否则使用默认路径
    arg.unwrap_or_else(AppConfig::default_path)
}
