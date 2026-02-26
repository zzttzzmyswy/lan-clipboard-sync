; LAN Clipboard Sync - Windows Installer Script
; Requires Inno Setup (https://jrsoftware.org/isdl.php)

#define AppName "LAN Clipboard Sync"
#define AppVersion "0.1.0"
#define AppPublisher "Your Name"
#define AppURL "https://github.com/zzttzzmyswy/lan-clipboard-sync"
#define AppExeName "lan-clipboard-sync.exe"

[Setup]
; 应用程序基本信息
AppId={{A8B2F3C4-5D6E-7F8A-9B0C-1D2E3F4A5B6C}}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL={#AppURL}
DefaultDirName={autopf}\LAN Clipboard Sync
DefaultGroupName={#AppName}
AllowNoIcons=yes
OutputBaseFilename=lan-clipboard-sync-{#AppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64
ArchitecturesAllowed=x64

; 安装程序语言
[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Languages\ChineseSimp.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

; 任务选择
[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加图标:"; Flags: unchecked
Name: "quicklaunchicon"; Description: "创建快速启动快捷方式"; GroupDescription: "附加图标:"; Flags: unchecked; OnlyBelowVersion: 6.1
Name: "autostart"; Description: "开机自动启动"; GroupDescription: "其他任务:"; Flags: unchecked

; 文件安装
[Files]
; 主程序
Source: "target\release\lan-clipboard-sync.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion

; 配置文件模板
Source: "config-template.toml"; DestDir: "{app}"; Flags: ignoreversion

; 协议文件
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion

; 图标（如果有的话）
; Source: "icons\app.ico"; DestDir: "{app}"; Flags: ignoreversion

; 注意：不要安装以下文件，它们是开发/测试文件
; Source: "target\release\*.pdb"; DestDir: "{app}\pdb"; Flags: ignoreversion recursesubdirs createallsubdirs

; 快捷方式
[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\配置文件"; Filename: "{app}\config.toml"
Name: "{group}\卸载 {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon
Name: "{userappdata}\Microsoft\Internet Explorer\Quick Launch\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: quicklaunchicon

; 运行任务
[Run]
Filename: "{app}\{#AppExeName}"; Description: "启动 {#AppName}"; Flags: nowait postinstall skipifsilent

; 注册表项（用于开机自启动）
[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#AppName}"; ValueData: """{app}\{#AppExeName}"""; Tasks: autostart; Flags: uninsdeletevalue

; 卸载时执行
[UninstallDelete]
Type: filesandordirs; Name: "{app}\config.toml"
Type: filesandordirs; Name: "{userappdata}\lan-clipboard-sync"

; 自定义代码
[Code]
// 检查是否已经有配置文件，如果没有则复制模板
procedure CurStepChanged(CurStep: TSetupStep);
var
  ConfigFile: String;
  TemplateFile: String;
begin
  if CurStep = ssPostInstall then
  begin
    ConfigFile := ExpandConstant('{userappdata}\lan-clipboard-sync\config.toml');
    TemplateFile := ExpandConstant('{app}\config-template.toml');

    if not FileExists(ConfigFile) then
    begin
      // 确保目录存在
      ForceDirectories(ExtractFilePath(ConfigFile));

      // 复制配置模板
      if FileCopy(TemplateFile, ConfigFile, False) then
      begin
        Log('已从模板创建配置文件: ' + ConfigFile);
      end
      else
      begin
        Log('无法创建配置文件');
      end;
    end;
  end;
end;

// 卸载时询问是否删除配置文件
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ConfigFile: String;
begin
  if CurUninstallStep = usUninstall then
  begin
    ConfigFile := ExpandConstant('{userappdata}\lan-clipboard-sync\config.toml');

    if FileExists(ConfigFile) then
    begin
      if MsgBox('是否要删除配置文件？'#13#13'配置文件路径: ' + ConfigFile,
                 mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
      begin
        DeleteFile(ConfigFile);
      end;
    end;
  end;
end;