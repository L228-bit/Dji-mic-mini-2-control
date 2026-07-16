# Dji-mic-mini-2-control

面向 macOS 的大疆麦克风控制工具，基于开源 DJI Mic USB 控制项目扩展，重点适配 DJI Mic Mini 2 双发射器使用场景。

应用提供中文界面、接收器与发射器状态读取、双发射器独立设置、音频输入输出切换、Voice Comfort 实时人声处理，以及接收器按键快捷键映射。

> 本项目为非官方社区项目，与 DJI（大疆创新）没有隶属或授权关系。修改设备参数前请确认设备型号和固件版本。

## 主要功能

- 中文 macOS 图形界面。
- 读取接收器、两个发射器的连接状态、序列号、电量和固件信息。
- 双发射器独立控制降噪、人声音色等支持的参数。
- 修复双发射器同时连接时部分开关状态反复跳动的问题。
- 支持 DJI Mic Mini 2 多色前盖外观显示。
- 在应用内选择 macOS 音频输入和输出设备。
- Voice Comfort 实时人声柔化、齿音控制、动态压缩和输出增益。
- 将 DJI 接收器短按映射为 `Fn + Control`，并阻止其改变系统音量。
- macOS 菜单栏运行、开机自启和设备快捷控制。

## 界面与技术栈

- 前端：Svelte 5 + Vite
- 桌面框架：Tauri 2
- 设备通信：Rust + USB/HID
- 人声处理：JUCE 原生音频引擎
- 支持平台：当前定制版本以 Apple Silicon macOS 为主；上游协议和 CLI 仍保留跨平台结构

## 开发运行

环境要求：

- Rust 1.77 或更高版本
- Node.js 18 或更高版本
- npm
- macOS 构建 Voice Comfort 时需要 CMake、Xcode Command Line Tools 和本地 JUCE 源码

安装前端依赖并启动开发版：

```bash
cd gui
npm install
npm run tauri dev
```

构建 macOS 应用：

```bash
cd gui
npx tauri build --bundles app
```

命令行工具示例：

```bash
cargo run -p cli -- list
cargo run -p cli -- status --json
cargo run -p cli -- set noise-cancel strong
```

协议研究记录见 [PROTOCOL.md](PROTOCOL.md)。

## macOS 权限

接收器按键映射需要在“系统设置 → 隐私与安全性”中允许应用使用：

- 辅助功能
- 输入监控

Voice Comfort 需要麦克风权限，并依赖 BlackHole 等虚拟音频设备完成系统音频路由。

未使用 Apple Developer ID 签名的本地构建可能被 Gatekeeper 拦截。确认应用来源后可执行：

```bash
xattr -rd com.apple.quarantine "/Applications/大疆麦克风控制.app"
```

## 项目结构

```text
crates/protocol                  DJI Mic 协议、数据帧和设备模型
crates/device                    USB 设备发现与通信
crates/cli                       命令行控制工具
gui                              Tauri + Svelte 图形界面
native/voice-comfort-engine      JUCE Voice Comfort 音频引擎
packaging                        Linux 打包与 udev 文件
tools                            本项目辅助工具
```

## 致谢与来源

本项目是在以下三位作者的公开项目和研究基础上继续开发的：

1. [ShadowBitBasher](https://github.com/ShadowBitBasher) — [DJI-Mic-Control](https://github.com/ShadowBitBasher/DJI-Mic-Control)：提供核心 USB 控制协议、Rust 设备层、CLI 和跨平台 GUI 基础。
2. [hueyluox](https://github.com/hueyluox) — [dji-mic-command](https://github.com/hueyluox/dji-mic-command)：提供 DJI 接收器按键事件与 macOS 快捷键映射方面的研究参考。
3. [Jayaway](https://github.com/Jayaway) — [Vibe-Coding-for-DJI-Mic](https://github.com/Jayaway/Vibe-Coding-for-DJI-Mic)：提供 DJI Mic 在 macOS 上进行快捷操作和语音工作流实验的思路参考。

感谢上述作者公开代码和研究过程。各上游项目的版权及许可归原作者所有；复用或分发时请同时遵守其各自仓库中的许可证。

## 许可证

当前仓库沿用上游项目附带的 [LICENSE](LICENSE)。第三方依赖及参考项目保留各自许可证和版权声明。
