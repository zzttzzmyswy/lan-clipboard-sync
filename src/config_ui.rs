//! 配置 UI 模块：基于 egui 的简单配置编辑界面。

use crate::config::{AppConfig, PeerConfig};
use eframe::egui;
use std::path::PathBuf;

/// 内嵌中文字体（Noto Sans SC），配置 UI 启动时设置。
fn setup_chinese_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "NotoSansSC".to_owned(),
        egui::FontData::from_static(include_bytes!("../resources/noto_sans_sc.ttf")),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "NotoSansSC".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("NotoSansSC".to_owned());
    ctx.set_fonts(fonts);
}

/// 构建 NativeOptions，在 Linux 下允许非主线程创建事件循环。
fn native_options() -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_inner_size([420.0, 380.0])
        .with_title("LAN 剪贴板同步 - 配置");

    #[cfg(target_os = "linux")]
    let event_loop_builder = {
        Some(Box::new(|builder: &mut winit::event_loop::EventLoopBuilder<eframe::UserEvent>| {
            use winit::platform::wayland::EventLoopBuilderExtWayland;
            builder.with_any_thread(true);
        }) as eframe::EventLoopBuilderHook)
    };
    #[cfg(not(target_os = "linux"))]
    let event_loop_builder = None;

    eframe::NativeOptions {
        viewport,
        event_loop_builder,
        ..Default::default()
    }
}

/// 配置编辑器应用，在独立窗口中运行。
pub struct ConfigApp {
    config_path: PathBuf,
    listen_port: String,
    secret_key: String,
    max_file_size: String,
    peers: Vec<(String, String)>,
    message: Option<Message>,
}

#[derive(Clone)]
enum Message {
    Success(String),
    Error(String),
}

impl ConfigApp {
    pub fn new(config_path: PathBuf) -> Self {
        let config = AppConfig::load(config_path.clone()).unwrap_or_else(|e| {
            tracing::warn!("failed to load config, using defaults: {}", e);
            AppConfig {
                listen_port: 5000,
                secret_key: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
                max_file_size: 10 * 1024 * 1024,
                peers: vec![],
            }
        });
        Self {
            config_path,
            listen_port: config.listen_port.to_string(),
            secret_key: config.secret_key.clone(),
            max_file_size: config.max_file_size.to_string(),
            peers: config
                .peers
                .iter()
                .map(|p| (p.host.clone(), p.port.to_string()))
                .collect(),
            message: None,
        }
    }

    fn collect_config(&self) -> Result<AppConfig, String> {
        let listen_port: u16 = self.listen_port.trim().parse().map_err(|_| "监听端口必须是 1-65535 的数字")?;
        let max_file_size: u64 = self.max_file_size.trim().parse().map_err(|_| "最大文件大小必须是有效的数字（字节）")?;
        let mut peers = Vec::new();
        for (i, (host, port_str)) in self.peers.iter().enumerate() {
            let host = host.trim().to_string();
            if host.is_empty() {
                continue;
            }
            let port: u16 = port_str.trim().parse().map_err(|_| {
                format!("对端 #{} 的端口必须是有效数字", i + 1)
            })?;
            peers.push(PeerConfig { host, port });
        }
        let config = AppConfig {
            listen_port,
            secret_key: self.secret_key.trim().to_string(),
            max_file_size,
            peers,
        };
        config.validate().map_err(|e| e.to_string())?;
        Ok(config)
    }

    fn save(&mut self) {
        match self.collect_config() {
            Ok(cfg) => match cfg.save(&self.config_path) {
                Ok(()) => {
                    self.message = Some(Message::Success(
                        "配置已保存。重启程序后生效。".to_string(),
                    ));
                }
                Err(e) => {
                    self.message = Some(Message::Error(format!("保存失败: {}", e)));
                }
            },
            Err(e) => {
                self.message = Some(Message::Error(e));
            }
        }
    }
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 底部固定栏：保存按钮和提示信息，确保始终可见
        egui::TopBottomPanel::bottom("config_bottom")
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some(ref msg) = self.message {
                        match msg {
                            Message::Success(s) => {
                                ui.colored_label(egui::Color32::GREEN, s);
                            }
                            Message::Error(s) => {
                                ui.colored_label(egui::Color32::RED, s);
                            }
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("保存").clicked() {
                            self.save();
                        }
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("LAN 剪贴板同步 - 配置");
            ui.add_space(8.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("监听端口:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.listen_port)
                            .desired_width(80.0)
                            .hint_text("5000"),
                    );
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("密钥 (hex):");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.secret_key)
                            .desired_width(300.0)
                            .hint_text("32+ 十六进制字符"),
                    );
                });
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("最大文件 (字节):");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.max_file_size)
                            .desired_width(120.0)
                            .hint_text("10485760"),
                    );
                });
                ui.add_space(12.0);

                ui.separator();
                ui.label("对端设备");
                ui.add_space(4.0);

                let mut to_remove = None;
                for (i, (host, port)) in self.peers.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label("IP:");
                        ui.add(egui::TextEdit::singleline(host).desired_width(120.0));
                        ui.label("端口:");
                        ui.add(egui::TextEdit::singleline(port).desired_width(60.0));
                        if ui.button("删除").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                if let Some(i) = to_remove {
                    self.peers.remove(i);
                }

                if ui.button("＋ 添加对端").clicked() {
                    self.peers.push(("".to_string(), "5000".to_string()));
                }
            });
        });
    }
}

/// 在独立窗口中运行配置 UI（阻塞直到窗口关闭）。
pub fn run(config_path: PathBuf) {
    let options = native_options();

    let _ = eframe::run_native(
        "LAN Clipboard Sync Config",
        options,
        Box::new(move |cc| {
            setup_chinese_font(&cc.egui_ctx);
            Ok(Box::new(ConfigApp::new(config_path)))
        }),
    );
}
