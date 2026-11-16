# Codex Window Capture

[English](README.md) | **简体中文**

为部分 **Windows 10 上 Computer Use 能找到窗口、能读控件, 但原生截图反复超时** 的用户提供前台单窗口截图。常见报错是 `FrameArrived timed out` 或 `window capture timed out`。原生截图正常的电脑无需安装。本项目在 Windows 10 22H2 x64、Sky 0.7.18 上验证过。

Codex 继续通过 Sky 读取和操作可访问性控件。需要画面时, 独立程序用 Windows Graphics Capture 取帧, 回调交出帧后立即返回, 由接收线程转换成 PNG。程序不修改 Sky 或桌面应用。它适用于会读取 `AGENTS.md` 且能运行本地 PowerShell 的代理。

## 安装

1. 从本仓库的 [Releases](https://github.com/senzyo/codex-window-capture/releases/latest) 下载 `codex-window-capture-windows-x64.zip`。便携包内含 EXE、脚本和规则, 无需 Rust。在下载目录用 PowerShell 解压:

   ```powershell
   $install = Join-Path $env:LOCALAPPDATA 'codex-window-capture'
   Expand-Archive -LiteralPath .\codex-window-capture-windows-x64.zip -DestinationPath $install
   ```

2. 把仓库当前 [AGENTS.md](https://github.com/senzyo/codex-window-capture/blob/main/AGENTS.md) 的**英文规则**写入用户级 `%USERPROFILE%\.codex\AGENTS.md` (自定义 `CODEX_HOME` 时使用该目录下的文件) 。已有本工具的旧规则时将其替换, 保留其他指令。旧版 Release ZIP 可能仍包含早期规则。若选了其他安装位置, 把规则中的脚本路径一并改好。
3. 在新的 Codex 会话里照常要求检查某个窗口。代理会直接使用本地截图路径, PNG 保存在安装目录的 `screenshots\`。无需加载额外技能。

## 手动验证

```powershell
& "$env:LOCALAPPDATA\codex-window-capture\scripts\capture-window.ps1" `
    -ApplicationPath "$env:WINDIR\System32\notepad.exe" -WindowTitle '无标题 - 记事本'
```

把程序路径和完整标题换成实际值。手动从终端执行时, 回车后在两秒内切回目标窗口。成功会打印 PNG 路径。窗口须在交互桌面上可见、位于前台且未最小化；同一程序有两个相同标题的可见窗口时, 工具会拒绝截图。宿主的桌面访问授权仍然适用。

此工具只获取图像。它不生成 Sky 的截图 ID, 依赖该 ID 的坐标点击仍受原生接口限制。只下载 `window-capture.exe` 的用户也可在其目录运行 `.\window-capture.exe <程序绝对路径> <完整标题> <新PNG路径>`。

## 执行上下文与排查

在命令沙盒外的交互用户上下文运行脚本。有沙盒且允许提权的 Codex 会话中, 为 `exec_command` 设置 `sandbox_permissions:"require_escalated"`。无沙盒或全访问会话中直接运行, 省略该参数。该参数请求在 Codex 沙盒外执行, 不代表请求 Windows 管理员提权。会话不允许退出沙盒时, 应报告限制, 不绕过限制。

`No visible foreground window matched the specified application and title` 也可能表示命令所在的执行上下文无法访问目标窗口。若 Sky 能读到控件, 先核对前台窗口、实际程序路径和完整标题, 再检查脚本的运行用户与桌面上下文。Windows 10 22H2 上的一次本地对照中, 相同脚本和目标在 `CodexSandboxOnline` 下失败, 在沙盒外的交互用户下成功生成 PNG；这不能证明故障由桌面应用更新引起。应继续排查脚本的执行上下文, 不自动退回此前已超时的原生截图。

## 构建与发布

在 Windows x64 上安装 Rust MSVC 工具链、Visual Studio C++ Build Tools 和 Windows SDK。在仓库根目录运行:

```powershell
cargo build --locked --release --target x86_64-pc-windows-msvc
```

仓库中的脚本也会查找这份本地构建。维护者将版本标签 (例如 `v0.1.0`) 推送到 GitHub 后, [发布工作流](https://github.com/senzyo/codex-window-capture/blob/main/.github/workflows/release.yml) 会构建 Windows x64 EXE, 发布便携 ZIP 和 `SHA256SUMS.txt`。可用 `Get-FileHash -Algorithm SHA256` 对照下载文件。

项目采用 [MIT 许可证](LICENSE), 依赖声明见 [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)。
