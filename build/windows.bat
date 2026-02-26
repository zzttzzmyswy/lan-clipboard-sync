@echo off
REM LAN Clipboard Sync - Windows 构建脚本

set VERSION=0.1.0
set PROJECT_ROOT=%~dp0..
set INSTALLER_DIR=%PROJECT_ROOT%\installer\windows
set OUTPUT_DIR=%PROJECT_ROOT%\dist\windows

echo ==========================================
echo LAN Clipboard Sync - Windows 构建脚本
echo 版本: %VERSION%
echo ==========================================
echo.

REM 检查 Rust 工具链
echo 检查 Rust 工具链...
where cargo >nul 2>&1
if %errorlevel% neq 0 (
    echo 错误: 未找到 Rust 工具链
    echo 请访问: https://www.rust-lang.org/tools/install
    exit /b 1
)
echo Rust 工具链检查完成
echo.

REM 检查 Inno Setup 编译器
echo 检查 Inno Setup 编译器...
where ISCC.exe >nul 2>&1
if %errorlevel% neq 0 (
    echo 警告: 未找到 Inno Setup 编译器 (ISCC.exe)
    echo 请从以下地址下载并安装: https://jrsoftware.org/isdl.php
    echo.
    set /p CONTINUE="是否继续构建可执行文件（不创建安装程序）？ (y/n): "
    if /i not "%CONTINUE%"=="y" (
        exit /b 1
    )
    set CREATE_INSTALLER=false
) else (
    set CREATE_INSTALLER=true
)
echo.

REM 创建输出目录
echo 创建输出目录...
if not exist "%OUTPUT_DIR%" mkdir "%OUTPUT_DIR%"
echo.

REM 构建 Windows 可执行文件
echo ==========================================
echo 开始构建 Windows 可执行文件...
echo ==========================================
cd /d "%PROJECT_ROOT%"

echo 设置目标架构: x86_64-pc-windows-msvc

REM 检查是否已安装 Windows 目标
rustup target list --installed | findstr "x86_64-pc-windows-msvc" >nul 2>&1
if %errorlevel% neq 0 (
    echo 安装 Windows 目标...
    rustup target add x86_64-pc-windows-msvc
)

echo 开始编译...
cargo build --release
if %errorlevel% neq 0 (
    echo 构建失败！
    exit /b 1
)

echo 构建完成！
echo.

REM 复制可执行文件到安装程序目录
echo 复制文件到安装程序目录...
copy "target\release\lan-clipboard-sync.exe" "%INSTALLER_DIR%\"
echo.

REM 创建安装程序
if "%CREATE_INSTALLER%"=="true" (
    echo ==========================================
    echo 创建安装程序...
    echo ==========================================
    
    cd /d "%INSTALLER_DIR%"
    
    REM 运行 Inno Setup 编译器
    ISCC.exe lan-clipboard-sync.iss
    if %errorlevel% neq 0 (
        echo 安装程序创建失败！
        exit /b 1
    )
    
    echo.
    echo 安装程序创建完成！
    echo.
) else (
    echo 跳过安装程序创建
    echo.
    echo 可执行文件位置: %INSTALLER_DIR%\lan-clipboard-sync.exe
    echo.
)

REM 查找生成的安装程序
if "%CREATE_INSTALLER%"=="true" (
    echo ==========================================
    echo 生成的文件:
    echo ==========================================
    
    if exist "%INSTALLER_DIR%\lan-clipboard-sync-%VERSION%-setup.exe" (
        move "%INSTALLER_DIR%\lan-clipboard-sync-%VERSION%-setup.exe" "%OUTPUT_DIR%\"
        echo ✓ %OUTPUT_DIR%\lan-clipboard-sync-%VERSION%-setup.exe
    )
    
    for %%f in ("%INSTALLER_DIR%\*.exe") do (
        echo   %%~nxf
    )
    echo.
)

echo ==========================================
echo Windows 构建完成！
echo ==========================================
echo.
echo 输出目录: %OUTPUT_DIR%
echo.

pause