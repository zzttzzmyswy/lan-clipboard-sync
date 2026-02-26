//! 构建脚本：将 resources/icon.png 转为 ICO 并嵌入 Windows 可执行文件。

fn main() {
    #[cfg(target_os = "windows")]
    embed_windows_icon();
}

#[cfg(target_os = "windows")]
fn embed_windows_icon() {
    use std::env;
    use std::fs::File;
    use std::io::BufReader;
    use std::path::PathBuf;

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let png_path = manifest_dir.join("resources").join("icon.png");
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ico_path = out_dir.join("tray_icon.ico");

    // 读取 PNG 并转换为 ICO
    let png_file = File::open(&png_path).expect("无法打开 resources/icon.png");
    let png_reader = BufReader::new(png_file);
    let image = ico::IconImage::read_png(png_reader).expect("无法解析 icon.png");

    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    icon_dir.add_entry(ico::IconDirEntry::encode(&image).expect("无法编码图标"));

    let ico_file = File::create(&ico_path).expect("无法创建 ico 文件");
    icon_dir.write(ico_file).expect("无法写入 ico 文件");

    // 嵌入到 Windows 可执行文件，使用 tray-default 名称以便 tray-item 加载
    let mut res = winres::WindowsResource::new();
    res.set_icon_with_id(&ico_path.to_string_lossy(), "tray-default");
    res.compile().expect("无法编译 Windows 资源");
}
