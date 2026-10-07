<div align="center">

# CC Switch

### Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes Agent、Pi、MiniMax Code 的全方位管理工具

**一键切换 API 供应商，统一管理 MCP、Skills 与提示词，不用再手改 JSON / TOML / YAML 配置文件。**

[![Version](https://img.shields.io/github/v/release/farion1231/cc-switch?color=blue&label=version)](https://github.com/farion1231/cc-switch/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/farion1231/cc-switch/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/farion1231/cc-switch/total)](https://github.com/farion1231/cc-switch/releases/latest)

<a href="https://trendshift.io/repositories/15372" target="_blank"><img src="https://trendshift.io/api/badge/repositories/15372" alt="farion1231%2Fcc-switch | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>
<a href="https://www.star-history.com/#farion1231/cc-switch&Date"><picture><source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/badge?repo=farion1231/cc-switch&theme=dark" /><img alt="Star History Rank" src="https://api.star-history.com/badge?repo=farion1231/cc-switch" width="196" height="55" /></picture></a>

本仓库是 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 的精简版，只管理配置文件。上游官网 ccswitch.io 以及 Homebrew、AUR 上的安装包是上游版本。

[English](README.md) | 中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [更新日志](CHANGELOG.md)

**[下载安装](#下载安装) · [快速开始](#快速开始) · [功能特性](#功能特性) · [常见问题](#常见问题) · [用户手册](docs/user-manual/zh/README.md)**

</div>

## 为什么选择 CC Switch？

Claude Code、Codex、Gemini CLI 等 AI 编程工具各有各的配置格式。换一个 API 供应商，就得手动改 JSON、TOML、YAML 或 `.env` 文件；MCP、Skills 和提示词也要在每个工具里分别维护。

**CC Switch** 把这些工作集中到一个桌面应用里：选一个预设、填入 Key，一键即可切换，原有配置不会丢失。

- **一个应用，九个工具** — Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes、Pi、MiniMax Code
- **告别手动编辑** — 60+ 供应商预设，包括 AWS Bedrock、NVIDIA NIM、OpenRouter、DeepSeek、Kimi 等
- **只管配置文件** — CC Switch 只写各工具自己的配置文件，不在本机运行代理，也不转发你的请求
- **MCP、Skills 与提示词集中管理** — MCP 和 Skills 添加一次，按工具勾选同步；提示词按工具分别维护
- **用量与额度一目了然** — 从各工具的本地会话记录统计 Token 用量和花费，供应商卡片和托盘上直接显示订阅额度与余额
- **跨平台** — 基于 Tauri 2 构建的原生桌面应用，支持 Windows、macOS 和 Linux

## 界面预览

|                  主界面                   |                  添加供应商                  |
| :---------------------------------------: | :------------------------------------------: |
| ![主界面](assets/screenshots/main-zh.png) | ![添加供应商](assets/screenshots/add-zh.png) |

## 下载安装

### 系统要求

- **Windows**：Windows 10 及以上
- **macOS**：macOS 12 (Monterey) 及以上
- **Linux**：x86_64 或 ARM64，需要 glibc 2.35+ 和 WebKitGTK 4.1，例如 Ubuntu 22.04+、Debian 12+ 及较新的 Fedora；RHEL / Rocky / Alma 8–9 暂不支持

### Windows 用户

从 [Releases](../../releases) 页面下载最新版本的 `CC-Switch-v{版本号}-Windows.msi` 安装包或 `CC-Switch-v{版本号}-Windows-Portable.zip` 绿色版。ARM 版 Windows 请下载 `CC-Switch-v{版本号}-Windows-arm64.msi` 或 `CC-Switch-v{版本号}-Windows-arm64-Portable.zip`。

### macOS 用户

从 [Releases](../../releases) 页面下载 `CC-Switch-v{版本号}-macOS.dmg`（推荐）或 `.zip`。这是 Universal 通用包，Apple Silicon 和 Intel Mac 均可原生运行。

> **注意**：macOS 包是否经过 Apple 公证，每个版本的 Release 说明里会写明。没有公证的版本首次打开会被系统拦截，在终端运行 `xattr -dr com.apple.quarantine "/Applications/ccs-lite.app"` 后即可打开。

### Linux 用户

从 [Releases](../../releases) 页面下载最新版本的 Linux 安装包：

- `CC-Switch-v{版本号}-Linux-x86_64.deb` / `-Linux-arm64.deb`（Debian/Ubuntu）
- `CC-Switch-v{版本号}-Linux-x86_64.rpm` / `-Linux-arm64.rpm`（Fedora 等提供 WebKitGTK 4.1 的 RPM 发行版）
- `CC-Switch-v{版本号}-Linux-x86_64.AppImage` / `-Linux-arm64.AppImage`（满足上述系统要求的发行版）

> **Flatpak**：官方 Release 不包含 Flatpak 包。如需使用，可从 `.deb` 自行构建 — 参见 [`flatpak/README.md`](flatpak/README.md)。

## 快速开始

### 基本使用

1. **添加供应商**：点击工具栏的“添加新供应商”（+ 按钮）→ 选择预设或创建自定义配置
2. **切换供应商**：
   - 主界面：选择供应商 → 点击“启用”（OpenCode、OpenClaw、Hermes、MiniMax Code 的按钮为“添加”；这四个工具和 Pi 是共存式工具，可以同时添加多个供应商）
   - 系统托盘：直接点击供应商名称（支持 Claude Code、Codex、Gemini CLI、Grok Build）
3. **生效方式**：Claude Code 无需重启；Codex、Gemini CLI、Grok Build 需重启终端或对应的 CLI 工具（详见常见问题）
4. **恢复官方登录**：切换到列表中自带的官方供应商（如“Claude Official”），重启工具后按照其登录/OAuth 流程操作

### MCP、提示词、Skills、项目与会话

- **MCP**：点击“MCP 管理”按钮 → 通过模板或自定义配置添加服务器（或“导入已有”）→ 切换各工具的同步开关
- **提示词**：点击“提示词” → 使用 Markdown 编辑器创建提示词 → 启用后写入该工具的提示词文件
- **Skills**：点击“Skills” →“发现技能” → 搜索 skills.sh 或浏览 GitHub 仓库 → 一键安装到支持的工具
- **项目**：在 Claude Code 或 Codex 页面，打开主页顶部的项目切换器 →“新建项目”，把当前配置保存下来，之后从切换器里选择即可整套切换
- **会话**：点击“会话管理” → 浏览、搜索和恢复各工具的会话历史

> **注意**：首次启动时，CC Switch 会自动把 Claude Code、Codex、Gemini CLI、Grok Build 的现有配置导入为名为 `default` 的供应商，并为这几个工具添加官方供应商，原有配置不会丢失。

各项功能的详细用法请查阅 **[用户手册](docs/user-manual/zh/README.md)**，涵盖供应商管理、MCP/提示词/Skills、用量统计等全部功能。

## 功能特性

[完整更新日志](CHANGELOG.md) | [发布说明](docs/release-notes/v3.20.4-zh.md)

### 各工具支持的功能

| 工具 | 供应商 | 托盘切换 | MCP | Skills | 提示词 | 会话 | 用量统计 |
| --- | --- | :---: | :---: | :---: | --- | :---: | :---: |
| Claude Code | 切换 | ✓ | ✓ | ✓ | CLAUDE.md | ✓ | ✓ |
| Codex | 切换 | ✓ | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| Gemini CLI | 切换 | ✓ | ✓ | ✓ | GEMINI.md | ✓ | ✓ |
| Grok Build | 切换 | ✓ | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| OpenCode | 共存 | – | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| OpenClaw | 共存 | – | – | – | 工作区编辑器 | ✓ | – |
| Hermes | 共存 | – | ✓ | ✓ | 记忆管理 | ✓ | – |
| Pi | 共存 | – | – | ✓ | AGENTS.md、SYSTEM.md、提示词模板 | ✓ | ✓ |
| MiniMax Code | 共存 | – | ✓ | ✓ | AGENTS.md | ✓ | ✓ |

- **切换**：同一时间只启用一个供应商；**共存**：多个供应商同时写入工具自身的配置，在工具里选择使用。
- **会话**：浏览、搜索会话历史，复制恢复命令继续对话（OpenClaw、Hermes 的会话暂不支持恢复）。Hermes 的会话需要在会话管理里选择“全部”查看。
- **用量统计**：从各工具的本地会话记录统计。

### 供应商管理

- **60+ 供应商预设** — 选择预设、填入 Key 即可添加，也可以创建自定义配置
- **只改关键字段** — 切换时只替换请求地址、Key、模型等连接信息，插件、Hook、MCP、你自己加的设置和注释都原样保留
- **项目** — 把 Claude Code 或 Codex 当前的供应商、MCP、Skills 和提示词文件保存为一个项目，之后在主页顶部的项目切换器或托盘里一键整套切换；切到其他项目时，当前状态会自动存回原项目
- **Codex 多 ChatGPT 账号** — 在「授权中心」登录多个 ChatGPT 账号，再为每张 OpenAI Official 卡片选择使用的账号，切换卡片即切换 Codex 的登录
- **通用供应商** — 一份配置同步到 Claude Code、Codex 和 Gemini CLI
- 一键切换、系统托盘快速切换（Claude Code、Codex、Gemini CLI、Grok Build）、拖拽排序、导入导出

### MCP、提示词与 Skills

- **统一 MCP 面板** — 一处管理所有 MCP 服务器，按工具勾选同步，支持从各工具导入现有配置，支持 Deep Link 导入
- **提示词** — 按工具分别管理的提示词库，使用 Markdown 编辑器；启用后写入该工具的提示词文件（CLAUDE.md / AGENTS.md / GEMINI.md），启用前会先把文件里原有的内容存回提示词库，不会丢失。Pi 还可以编辑 SYSTEM.md、APPEND_SYSTEM.md 和提示词模板
- **Skills** — 搜索 skills.sh，或从 GitHub 仓库、ZIP 文件一键安装；检查更新并一键全部更新；通过软链接或文件复制同步到各工具，存储位置可选 `~/.agents/skills`
- 三个面板都支持搜索，MCP 和 Skills 还可以按工具一键全部启用或停用

### 用量与成本追踪

- **用量仪表盘** — 默认自动扫描各工具的本地会话记录，按供应商和模型统计请求数、Token、缓存命中率和花费，提供趋势图和逐条请求日志
- **额度与余额** — 供应商卡片和托盘上直接显示官方订阅额度（Claude、ChatGPT、Gemini、SuperGrok）、Coding Plan 的 5 小时 / 周 / 月额度（Kimi、智谱 GLM、MiniMax、火山方舟等）和账户余额（DeepSeek、OpenRouter、硅基流动等），部分需要先在供应商卡片的“配置用量查询”里开启；其他供应商可以写自定义用量脚本
- **自定义定价** — 按模型设置单价，可以从 models.dev 导入

### 会话管理器与工作区

- **会话管理器** — 浏览、搜索各工具的会话历史，复制恢复命令继续对话；macOS 上可以一键在终端中恢复
- **工作区编辑器**（OpenClaw）— 编辑 Agent 文件（AGENTS.md、SOUL.md 等）和每日记忆
- **记忆管理**（Hermes）— 编辑 Hermes 的 MEMORY.md 和 USER.md

### 系统与平台

- **云同步** — 通过 WebDAV（坚果云、Nextcloud、群晖 NAS 等）或 S3 兼容存储（AWS S3、Cloudflare R2、阿里云 OSS、腾讯云 COS 等）在多台设备之间同步；也可以把 CC Switch 配置目录放到 Dropbox、OneDrive、iCloud 等网盘文件夹中
- **CLI 工具管理** — 在「关于」页查看 Claude Code、Codex 等命令行工具的当前版本和最新版本，一键安装、升级或全部升级，并诊断重复安装；Windows 上还能管理 WSL 里的工具（见常见问题）
- **Deep Link**（`ccswitch://`）— 通过链接一键导入供应商、MCP 服务器和提示词，或添加技能仓库
- **小工具** — 跳过 Claude Code 初次安装确认、隐藏 AI 署名、让 VS Code 的 Claude Code 插件随本软件切换供应商等
- 深色 / 浅色 / 跟随系统主题、开机自启、自动更新、原子写入、自动备份、国际化（简中/繁中/英/日）

## 常见问题

<details>
<summary><strong>CC Switch 支持哪些 AI 工具？</strong></summary>

CC Switch 支持九个工具：**Claude Code**、**Codex**、**Gemini CLI**、**Grok Build**、**OpenCode**、**OpenClaw**、**Hermes**、**Pi**、**MiniMax Code**。每个工具都有专属的供应商预设和配置管理，各自支持哪些功能见[各工具支持的功能](#各工具支持的功能)。

</details>

<details>
<summary><strong>切换供应商后需要重启终端吗？</strong></summary>

视工具而定：

- **Claude Code**：支持供应商数据的热切换，无需重启。
- **Codex、Gemini CLI、Grok Build**：需要重启终端或 CLI 工具才能生效（切换成功后会有提示）。
- **OpenCode、OpenClaw、Hermes、Pi、MiniMax Code**：这些是共存式工具，点击“添加”（Pi 为“启用”）会把供应商写入工具自身的配置、与其他供应商共存，之后在工具里选择要使用的模型即可。

</details>

<details>
<summary><strong>切换供应商会改掉我的插件、Hook 等设置吗？</strong></summary>

不会。Claude Code、Codex、Gemini CLI、Grok Build 切换供应商时，CC Switch 只替换配置文件里的**关键字段**：请求地址、Key、模型名和接口协议（Codex 还包括推理档位，Gemini CLI 还包括认证方式），以及少数跟着供应商走的兼容选项（如 Claude Code 的“禁用 Artifact 工具”、上下文窗口）。插件、Hook、权限、MCP、你自己加的环境变量、注释和排版都原样保留，对所有供应商生效。

这些共享设置可以直接在工具里改，或手动编辑配置文件；也可以在 CC Switch 里编辑任意一个供应商：编辑框显示的是“切到这个供应商之后配置文件的样子”，保存时关键字段存进这个供应商，其余改动写进配置文件，对所有供应商生效。

所以以前的“通用配置片段”已经不需要了，相关按钮已移除。升级前片段里的设置在切换时早已写进配置文件，会继续保留。CC Switch 第一次改写每个配置文件之前，还会把原文件备份到 `~/.cc-switch/backups/live-first-write/`。

</details>

<details>
<summary><strong>在工具里换了模型，切走再切回来怎么又变回去了？</strong></summary>

模型属于关键字段，归供应商所有。在工具里换的模型（如 Claude Code 的 `/model`）会一直生效到下次切换；切换时，配置文件里的模型会换成目标供应商保存的那个，CC Switch 不会把你在工具里换的模型存回原来的供应商。想长期使用某个模型，请在 CC Switch 里编辑这个供应商。

旧版本会在切走时把整份配置文件存回供应商，现在不再这样做：那样会把插件等共享设置冻结进某一个供应商，切到别的供应商时就丢了。

</details>

<details>
<summary><strong>为什么总有一个正在激活中的供应商无法删除？</strong></summary>

本软件的设计原则是“最小侵入性”，即使卸载本软件，也不会影响应用的正常使用。

所以对于同一时间只启用一个供应商的工具（Claude Code、Codex、Gemini CLI、Grok Build），系统总会保留一个正在激活中的配置，因为如果将所有配置全部删除，该应用将无法正常使用。OpenCode、OpenClaw、Hermes、Pi、MiniMax Code 等共存式工具不受此限制，可以直接删除任意供应商。如果你不常用某个工具，可以在设置中关掉它的显示。如果你想切换回官方登录，可以参考下条。

</details>

<details>
<summary><strong>如何切换回官方登录？</strong></summary>

Claude Code、Codex、Gemini CLI、Grok Build 的供应商列表里都自带一个官方供应商（**Claude Official**、**OpenAI Official**、**Google Official**、**Grok Official**），如果删掉了，可以从预设里重新添加。切换到官方供应商后，按照工具自身的登录流程操作（如 Claude Code 的 `/login`、Codex 的 `codex login`），之后便可以在官方供应商和第三方供应商之间随意切换。

Codex 还可以在 CC Switch 里用“使用 ChatGPT 登录”登录多个 ChatGPT 账号，再为每张 **OpenAI Official** 卡片选择“使用的账号”，多个 Plus、Pro 或 Team 账号之间一键切换；选择“跟随 Codex 登录”的卡片则沿用 Codex CLI 自己的登录。

</details>

<details>
<summary><strong>能在 Claude Code 里使用 OpenAI 兼容接口或本地模型吗？</strong></summary>

要看服务有没有提供工具需要的接口格式。CC Switch 只写配置文件，不转换接口格式：Claude Code 需要 Anthropic Messages 接口，Codex 和 Grok Build 需要 OpenAI Responses 接口，Gemini CLI 需要 Gemini API。很多供应商（DeepSeek、Kimi、智谱 GLM、MiniMax 等）都提供 Anthropic 兼容接口，对应的预设已经填好。只提供 Chat Completions 接口的服务，需要你自己运行一个格式转换代理，再把它的地址填为供应商的请求地址。

</details>

<details>
<summary><strong>“检测连通”通过了，为什么请求还是失败？</strong></summary>

供应商卡片上的“检测连通”只检查供应商地址能不能连上，不会发送真实的模型请求，所以验证不了 API Key 和模型名是否正确。请求失败时，请检查 Key、模型名，以及请求地址的接口格式是否是工具需要的（见上一条）。

</details>

<details>
<summary><strong>我的数据存储在哪里？</strong></summary>

默认都在用户主目录下的 `.cc-switch` 文件夹（Windows 为 `C:\Users\<用户名>\.cc-switch`）：

- **数据库**：`cc-switch.db`（SQLite — 供应商、MCP、提示词、Skills、项目、用量记录等）
- **本地设置**：`settings.json`（设备级设置，如各工具的配置目录、备份策略、云同步连接信息）
- **备份**：`backups/`（默认每 24 小时自动备份一次、保留最近 10 个，可在「设置 → 高级 → 备份与恢复」中调整）
- **Skills**：`skills/`（可在设置中改为 `~/.agents/skills`），默认通过软链接同步到各工具，失败时改为复制
- **技能备份**：`skill-backups/`（卸载或更新技能前自动创建，保留最近 20 个）
- **ChatGPT 账号凭据**：`codex_oauth_auth.json`
- **日志**：`logs/cc-switch.log` 和 `crash.log`，反馈问题时请附上
- **本机状态**：`live-state.json`（CC Switch 上一次往各工具配置文件里写了什么）、`codex-login-stash.json`（切到第三方时被移走的 Codex 官方登录，切回官方时还原）
- **配置文件原件**：`backups/live-first-write/`（CC Switch 第一次改写各工具配置文件之前的原文件）

在「设置 → 高级 → 配置文件目录」里修改“CC Switch 配置目录”后，除 `settings.json`、本机状态和配置文件原件以外的上述文件都改为存放在新目录。CC Switch 不会自动搬运已有文件，需要先手动复制过去。`settings.json`、本机状态和配置文件原件只属于这台电脑，始终在默认目录，也不参与云同步。

</details>

<details>
<summary><strong>在 Windows 上怎么管理 WSL 里的工具？</strong></summary>

CC Switch 不会自动识别 WSL。请在「设置 → 高级 → 配置文件目录 → 配置目录覆盖（高级）」里，把对应工具的目录改成 WSL 里的路径，例如 `\\wsl.localhost\Ubuntu\home\<用户名>\.claude`，保存后 CC Switch 就会读写 WSL 里的配置（Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes、Pi 支持设置）。设置之后，「关于」页也会在对应的 WSL 发行版里检测和升级该工具。

</details>

<details>
<summary><strong>有命令行版本或无界面版本吗？</strong></summary>

CC Switch 本身只提供需要图形界面的桌面版（系统要求见[下载安装](#下载安装)）。在服务器、SSH 远程或没有桌面环境的机器上，推荐使用社区维护的 **[CC Switch CLI](https://github.com/SaladDay/cc-switch-cli)**：它提供交互式终端界面（TUI）和命令行两种用法，支持 Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes、Pi，可以通过 Homebrew（`brew install cc-switch-cli`）或安装脚本安装。

CC Switch CLI 默认与桌面版共用数据目录 `~/.cc-switch`，也兼容桌面版的 WebDAV 同步。两个项目分别发版，CLI 版支持的数据库版本有时会落后于桌面版；遇到“数据库版本过新”的提示时，请升级 CLI 版，或等它跟进更新。

</details>

<details>
<summary><strong>Linux（Wayland + NVIDIA）：网页内容点不动、缩放后黑屏</strong></summary>

AppImage 会强制 `GDK_BACKEND=x11`（走 XWayland）以规避历史上的原生 Wayland 崩溃。但在较新的 Wayland + NVIDIA 环境下，这会导致网页内容区点不动（标题栏按钮仍可点）、窗口缩放后黑屏。可用内置的逃生开关切回原生 Wayland：

```bash
CC_SWITCH_GDK_BACKEND=wayland ./CC-Switch-*.AppImage
```

如果你是从桌面图标启动的，请把它写进 `.desktop` 的 `Exec=` 行（如 `env CC_SWITCH_GDK_BACKEND=wayland /path/to/AppImage`），或在会话环境中设置。该变量是通用的：在 tiling Wayland 合成器（sway/Hyprland）下若出现点击失效，可反过来设 `CC_SWITCH_GDK_BACKEND=x11`。不设置则保持默认行为。

</details>

更多问题请查看用户手册中的[常见问题](docs/user-manual/zh/5-faq/5.2-questions.md)。

## 贡献

欢迎提交 Issue 反馈问题和建议！新功能开发前，请先开 Issue 讨论实现方案，不适合项目的功能性 PR 有可能会被关闭。

开发环境、提交前检查和架构说明见 [CONTRIBUTING.md](CONTRIBUTING.md#贡献指南)；使用问题请先看 [SUPPORT.md](SUPPORT.md)；安全漏洞请按 [SECURITY.md](SECURITY.md) 私下报告。

**技术栈**：Tauri 2 · Rust · React 18 · TypeScript · SQLite

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=farion1231/cc-switch&type=Date)](https://www.star-history.com/#farion1231/cc-switch&Date)

## License

MIT © Jason Young
