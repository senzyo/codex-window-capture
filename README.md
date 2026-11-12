# Codex Window Capture

为部分 **Windows 10 上 Computer Use 能找到窗口、能读控件，但原生截图反复超时** 的用户提供前台单窗口截图。常见报错是 `FrameArrived timed out` 或 `window capture timed out`。原生截图正常的电脑无需安装。本项目在 Windows 10 22H2 x64、Sky 0.7.18 上验证过。

Codex 继续通过 Sky 读取和操作可访问性控件。需要画面时，独立程序用 Windows Graphics Capture 取帧，回调交出帧后立即返回，由接收线程转换成 PNG。程序不修改 Sky 或桌面应用。它适用于会读取 `AGENTS.md` 且能运行本地 PowerShell 的代理。

## 安装

1. 从本仓库的 **Releases** 下载 `codex-window-capture-windows-x64.zip`。便携包内含 EXE、脚本和规则，无需 Rust。在下载目录用 PowerShell 解压：

   ```powershell
   $install = Join-Path $env:LOCALAPPDATA 'codex-window-capture'
   Expand-Archive -LiteralPath .\codex-window-capture-windows-x64.zip -DestinationPath $install
   ```

2. 把包内 [AGENTS.md](AGENTS.md) 的**一行规则**追加到用户级 `%USERPROFILE%\.codex\AGENTS.md`（自定义 `CODEX_HOME` 时使用该目录下的文件）。保留原有内容。若选了其他安装位置，把规则中的脚本路径一并改好。
3. 在新的 Codex 会话里照常要求检查某个窗口。代理会直接使用本地截图路径，PNG 保存在安装目录的 `screenshots\`。无需加载额外技能。

## 手动验证

```powershell
& "$env:LOCALAPPDATA\codex-window-capture\scripts\capture-window.ps1" `
    -ApplicationPath "$env:WINDIR\System32\notepad.exe" -WindowTitle '无标题 - 记事本'
```

把程序路径和完整标题换成实际值。手动从终端执行时，回车后在两秒内切回目标窗口。成功会打印 PNG 路径。窗口须在交互桌面上可见、位于前台且未最小化；同一程序有两个相同标题的可见窗口时，工具会拒绝截图。宿主的桌面访问授权仍然适用。

此工具只获取图像。它不生成 Sky 的截图 ID，依赖该 ID 的坐标点击仍受原生接口限制。只下载 `window-capture.exe` 的用户也可直接调用 `window-capture.exe <程序绝对路径> <完整标题> <新PNG路径>`。

## 构建与发布

在 Windows x64 上安装 Rust MSVC 工具链、Visual Studio C++ Build Tools 和 Windows SDK。在仓库根目录运行：

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
```

仓库中的脚本也会查找这份本地构建。维护者将版本标签（例如 `v0.1.0`）推送到 GitHub 后，[发布工作流](.github/workflows/release.yml) 会构建 Windows x64 EXE，发布便携 ZIP 和 `SHA256SUMS.txt`。可用 `Get-FileHash -Algorithm SHA256` 对照下载文件。

项目采用 [MIT 许可证](LICENSE)，依赖声明见 [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)。
