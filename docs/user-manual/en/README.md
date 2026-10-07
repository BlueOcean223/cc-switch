# CC Switch User Manual

> All-in-One Assistant for Claude Code / Codex / Gemini CLI / Grok Build / OpenCode / OpenClaw / Hermes / Pi / MiniMax Code

## Table of Contents

```
📚 CC Switch User Manual
│
├── 1. Getting Started
│   ├── 1.1 Introduction
│   ├── 1.2 Installation Guide
│   ├── 1.3 Interface Overview
│   ├── 1.4 Quick Start
│   └── 1.5 Personalization
│
├── 2. Provider Management
│   ├── 2.1 Add Provider
│   ├── 2.2 Switch Provider
│   ├── 2.3 Edit Provider
│   ├── 2.4 Sort & Duplicate
│   └── 2.5 Usage Query
│
├── 3. Extensions
│   ├── 3.1 MCP Server Management
│   ├── 3.2 Prompts Management
│   ├── 3.3 Skills Management
│   ├── 3.4 Session Manager
│   └── 3.5 Workspace & Memory
│
├── 4. Usage & Connectivity
│   ├── 4.1 Usage Statistics
│   └── 4.2 Connectivity Check
│
└── 5. FAQ
    ├── 5.1 Configuration Files
    ├── 5.2 FAQ
    ├── 5.3 Deep Link Protocol
    └── 5.4 Environment Variable Conflicts
```

## File List

### 1. Getting Started

| File | Description |
|------|-------------|
| [1.1-introduction.md](./1-getting-started/1.1-introduction.md) | Introduction, core features, supported platforms |
| [1.2-installation.md](./1-getting-started/1.2-installation.md) | Windows/macOS/Linux installation guide |
| [1.3-interface.md](./1-getting-started/1.3-interface.md) | Interface layout, navigation bar, provider cards |
| [1.4-quickstart.md](./1-getting-started/1.4-quickstart.md) | 5-minute quick start tutorial |
| [1.5-settings.md](./1-getting-started/1.5-settings.md) | Language, theme, directories, cloud sync settings |

### 2. Provider Management

| File | Description |
|------|-------------|
| [2.1-add.md](./2-providers/2.1-add.md) | Using presets, custom configuration, universal providers |
| [2.2-switch.md](./2-providers/2.2-switch.md) | Main UI switching, tray switching, activation methods |
| [2.3-edit.md](./2-providers/2.3-edit.md) | Edit configuration, modify API Key, global settings and edit conflicts |
| [2.4-sort-duplicate.md](./2-providers/2.4-sort-duplicate.md) | Drag-to-reorder, duplicate provider, delete |
| [2.5-usage-query.md](./2-providers/2.5-usage-query.md) | Usage query, remaining balance, multi-plan display |

### 3. Extensions

| File | Description |
|------|-------------|
| [3.1-mcp.md](./3-extensions/3.1-mcp.md) | MCP protocol, add servers, app binding |
| [3.2-prompts.md](./3-extensions/3.2-prompts.md) | Create presets, activate/switch, smart backfill |
| [3.3-skills.md](./3-extensions/3.3-skills.md) | Discover skills, install/uninstall, repository management |
| [3.4-sessions.md](./3-extensions/3.4-sessions.md) | Session Manager: browse, search, resume, delete sessions |
| [3.5-workspace.md](./3-extensions/3.5-workspace.md) | Workspace files and daily memory (OpenClaw) |

### 4. Usage & Connectivity

| File | Description |
|------|-------------|
| [4.1-usage.md](./4-usage/4.1-usage.md) | Usage statistics, trend charts, pricing configuration |
| [4.2-connectivity-check.md](./4-usage/4.2-connectivity-check.md) | Connectivity check, check parameters |

### 5. FAQ

| File | Description |
|------|-------------|
| [5.1-config-files.md](./5-faq/5.1-config-files.md) | CC Switch storage, CLI configuration file formats |
| [5.2-questions.md](./5-faq/5.2-questions.md) | Frequently asked questions |
| [5.3-deeplink.md](./5-faq/5.3-deeplink.md) | Deep link protocol, generation and usage |
| [5.4-env-conflict.md](./5-faq/5.4-env-conflict.md) | Environment variable conflict detection and resolution |

## Quick Links

- **New users**: Start with [1.1 Introduction](./1-getting-started/1.1-introduction.md)
- **Installation issues**: See [1.2 Installation Guide](./1-getting-started/1.2-installation.md)
- **Configure providers**: See [2.1 Add Provider](./2-providers/2.1-add.md)
- **Having trouble**: See [5.2 FAQ](./5-faq/5.2-questions.md)

## Version Information

- Documentation version: v3.20.4
- Last updated: 2026-09-26
- Applicable to CC Switch v3.20.4+

### Recent Major Changes

- **New managed apps**: Grok Build (v3.18.0), Pi (v3.20.0), and MiniMax Code (v3.20.4), bringing the total to 9 managed apps — see [1.1 Introduction](./1-getting-started/1.1-introduction.md)
- **Local routing, failover, Claude Desktop, and GitHub Copilot / xAI sign-in removed**: CC Switch now only writes each tool's config files; Codex providers connect directly through native Responses endpoints — see [2.1 Add Provider](./2-providers/2.1-add.md)
- **Codex switching writes only config.toml**: third-party API keys are no longer written to `auth.json` (v3.20.1) — see [1.5 Personalization → Codex App Enhancements](./1-getting-started/1.5-settings.md#codex-app-enhancements)
- **Connectivity check replaces model test**: it only checks whether the address is reachable and no longer sends real model requests (v3.16.3) — see [4.2 Connectivity Check](./4-usage/4.2-connectivity-check.md)
- **Usage statistics from session logs**: usage is imported from each tool's local session logs — see [4.1 Usage Statistics](./4-usage/4.1-usage.md)
- **Cloud sync supports S3-compatible storage** — see [1.5 Personalization](./1-getting-started/1.5-settings.md)

## Contributing

Feel free to submit Issues or PRs to improve the documentation:

- [GitHub Issues](https://github.com/farion1231/cc-switch/issues)
- [GitHub Repository](https://github.com/farion1231/cc-switch)
