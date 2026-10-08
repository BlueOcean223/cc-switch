<div align="center">

# CC Switch

### Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes Agent、Pi、MiniMax Code のオールインワン管理ツール

**ワンクリックで API プロバイダを切り替え、MCP・Skills・プロンプトを一元管理。JSON / TOML / YAML の設定ファイルを手作業で編集する必要はもうありません。**

[![Version](https://img.shields.io/github/v/release/farion1231/cc-switch?color=blue&label=version)](https://github.com/farion1231/cc-switch/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/farion1231/cc-switch/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/farion1231/cc-switch/total)](https://github.com/farion1231/cc-switch/releases/latest)

<a href="https://trendshift.io/repositories/15372" target="_blank"><img src="https://trendshift.io/api/badge/repositories/15372" alt="farion1231%2Fcc-switch | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>
<a href="https://www.star-history.com/#farion1231/cc-switch&Date"><picture><source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/badge?repo=farion1231/cc-switch&theme=dark" /><img alt="Star History Rank" src="https://api.star-history.com/badge?repo=farion1231/cc-switch" width="196" height="55" /></picture></a>

このリポジトリは [farion1231/cc-switch](https://github.com/farion1231/cc-switch) の軽量版で、設定ファイルの管理だけを行います。上流の公式サイト ccswitch.io と Homebrew、AUR のパッケージは上流版です。

[English](README.md) | [中文](README_ZH.md) | 日本語 | [Deutsch](README_DE.md) | [Changelog](CHANGELOG.md)

**[ダウンロード](#ダウンロード--インストール) · [クイックスタート](#クイックスタート) · [特長](#特長) · [よくある質問](#よくある質問) · [ユーザーマニュアル](docs/user-manual/ja/README.md)**

</div>

## CC Switch を選ぶ理由

Claude Code、Codex、Gemini CLI などの AI コーディングツールは、それぞれ設定形式が異なります。API プロバイダを変えるたびに JSON、TOML、YAML、`.env` ファイルを手作業で編集しなければならず、MCP、Skills、プロンプトもツールごとに個別に管理する必要があります。

**CC Switch** は、こうした作業を 1 つのデスクトップアプリに集約します。プリセットを選んでキーを入力すれば、ワンクリックで切り替えられます。既存の設定が失われることもありません。

- **1 つのアプリで 9 つのツール** — Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes、Pi、MiniMax Code
- **手動編集は不要** — AWS Bedrock、NVIDIA NIM、OpenRouter、DeepSeek、Kimi など 60 以上のプロバイダプリセットを内蔵
- **設定ファイルだけを扱う** — CC Switch は各ツール自身の設定ファイルに書き込むだけで、ローカルでプロキシを動かしたり、リクエストを転送したりしません
- **MCP・Skills・プロンプトを一元管理** — MCP と Skills は一度追加すれば、ツールごとにチェックを入れて同期。プロンプトはツールごとに個別に管理
- **使用量とクォータをひと目で確認** — 各ツールのローカルセッション記録からトークン使用量と費用を集計。サブスクリプションのクォータと残高をプロバイダカードとトレイに直接表示
- **クロスプラットフォーム** — Tauri 2 で構築された Windows、macOS、Linux 対応のネイティブデスクトップアプリ

## スクリーンショット

|                  メイン画面                   |                  プロバイダ追加                  |
| :-------------------------------------------: | :----------------------------------------------: |
| ![メイン画面](assets/screenshots/main-ja.png) | ![プロバイダ追加](assets/screenshots/add-ja.png) |

## ダウンロード & インストール

### システム要件

- **Windows**: Windows 10 以上
- **macOS**: macOS 12 (Monterey) 以上
- **Linux**: x86_64 または ARM64、glibc 2.35 以上と WebKitGTK 4.1 が必要（例：Ubuntu 22.04+、Debian 12+、最近の Fedora）。RHEL / Rocky / Alma 8–9 は現在未対応

### Windows ユーザー

[Releases](../../releases) ページから最新版の `CC-Switch-v{version}-Windows.msi` インストーラー、またはポータブル版 `CC-Switch-v{version}-Windows-Portable.zip` をダウンロード。ARM 版 Windows では `CC-Switch-v{version}-Windows-arm64.msi` または `CC-Switch-v{version}-Windows-arm64-Portable.zip` をダウンロードしてください。

### macOS ユーザー

[Releases](../../releases) から `CC-Switch-v{version}-macOS.dmg`（推奨）または `.zip` をダウンロード。Apple Silicon と Intel Mac の両方でネイティブに動作する Universal ビルドです。

> **注意**: macOS パッケージが Apple の公証を受けているかどうかは、各バージョンのリリースノートに記載されています。公証されていないバージョンは初回起動時にシステムにブロックされるので、ターミナルで `xattr -dr com.apple.quarantine "/Applications/ccs-lite.app"` を実行してから開いてください。

### Linux ユーザー

[Releases](../../releases) から最新版の Linux ビルドをダウンロード：

- `CC-Switch-v{version}-Linux-x86_64.deb` / `-Linux-arm64.deb`（Debian/Ubuntu）
- `CC-Switch-v{version}-Linux-x86_64.rpm` / `-Linux-arm64.rpm`（WebKitGTK 4.1 を提供する Fedora などの RPM 系ディストリビューション）
- `CC-Switch-v{version}-Linux-x86_64.AppImage` / `-Linux-arm64.AppImage`（上記のシステム要件を満たすディストリビューション）

## クイックスタート

### 基本的な使い方

1. **プロバイダ追加**: ツールバーの「新しいプロバイダーを追加」（+ ボタン）をクリック → プリセットを選ぶかカスタム設定を作成
2. **プロバイダ切り替え**:
   - メイン UI: プロバイダを選択 → 「有効化」をクリック（OpenCode、OpenClaw、Hermes、MiniMax Code ではボタンが「追加」になります。この 4 つのツールと Pi は共存型のツールで、複数のプロバイダを同時に追加できます）
   - システムトレイ: プロバイダ名をクリック（Claude Code、Codex、Gemini CLI、Grok Build に対応）
3. **反映**: Claude Code は再起動不要。Codex、Gemini CLI、Grok Build はターミナルまたは CLI ツールを再起動（詳しくはよくある質問を参照）
4. **公式ログインに戻す**: リストに含まれている公式プロバイダ（例：「Claude Official」）に切り替え、ツールを再起動してログイン/OAuth フローを実行

### MCP、プロンプト、Skills、プロジェクト & セッション

- **MCP**: 「MCP 管理」ボタンをクリック → テンプレートまたはカスタム設定でサーバーを追加（または「既存をインポート」）→ ツールごとの同期をトグルで切り替え
- **プロンプト**: 「プロンプト」をクリック → Markdown エディタでプロンプトを作成 → 有効化すると、そのツールのプロンプトファイルに書き込み
- **Skills**: 「Skills」をクリック → 「スキルを発見」 → skills.sh を検索、または GitHub リポジトリを閲覧 → 対応ツールへワンクリックでインストール
- **プロジェクト**: Claude Code または Codex のページで、メインページ上部のプロジェクトスイッチャーを開く → 「新規プロジェクト」で現在の設定を保存。以降はスイッチャーから選ぶだけで設定一式を切り替え
- **セッション**: 「セッション管理」をクリック → 各ツールの会話履歴を閲覧・検索・復元

> **補足**: 初回起動時、CC Switch は Claude Code、Codex、Gemini CLI、Grok Build の既存設定を `default` という名前のプロバイダとして自動でインポートし、これらのツールに公式プロバイダを追加します。既存の設定が失われることはありません。

各機能の詳しい使い方については、**[ユーザーマニュアル](docs/user-manual/ja/README.md)** をご覧ください。プロバイダ管理、MCP/プロンプト/Skills、使用量統計など、すべての機能を網羅しています。

## 特長

[完全な更新履歴](CHANGELOG.md) | [リリースノート](docs/release-notes/v3.20.4-ja.md)

### ツール別の対応機能

| ツール | プロバイダ | トレイ切り替え | MCP | Skills | プロンプト | セッション | 使用量統計 |
| --- | --- | :---: | :---: | :---: | --- | :---: | :---: |
| Claude Code | 切り替え | ✓ | ✓ | ✓ | CLAUDE.md | ✓ | ✓ |
| Codex | 切り替え | ✓ | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| Gemini CLI | 切り替え | ✓ | ✓ | ✓ | GEMINI.md | ✓ | ✓ |
| Grok Build | 切り替え | ✓ | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| OpenCode | 共存 | – | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| OpenClaw | 共存 | – | – | – | ワークスペースエディタ | ✓ | – |
| Hermes | 共存 | – | ✓ | ✓ | メモリ | ✓ | – |
| Pi | 共存 | – | – | ✓ | AGENTS.md、SYSTEM.md、プロンプトテンプレート | ✓ | ✓ |
| MiniMax Code | 共存 | – | ✓ | ✓ | AGENTS.md | ✓ | ✓ |

- **切り替え**：同時に有効にできるプロバイダは 1 つだけです。**共存**：複数のプロバイダを同時にツール自身の設定に書き込み、ツール内で選んで使用します。
- **セッション**：会話履歴を閲覧・検索し、再開コマンドをコピーして会話を続けられます（OpenClaw と Hermes のセッションは現在、再開に対応していません）。Hermes のセッションは、セッション管理で「すべて」を選ぶと表示されます。
- **使用量統計**：各ツールのローカルセッション記録から集計します。

### プロバイダ管理

- **60 以上のプロバイダプリセット** — プリセットを選んでキーを入力するだけで追加。カスタム設定の作成も可能
- **主要フィールドだけを変更** — 切り替え時に置き換えるのはリクエスト先アドレス、キー、モデルなどの接続情報だけ。プラグイン、フック、MCP、自分で追加した設定やコメントはそのまま残ります
- **プロジェクト** — Claude Code または Codex の現在のプロバイダ、MCP、Skills、プロンプトファイルを 1 つのプロジェクトとして保存。以降はメインページ上部のプロジェクトスイッチャーやトレイから設定一式をワンクリックで切り替え。別のプロジェクトに切り替えると、現在の状態は自動的に元のプロジェクトへ保存
- **Codex で複数の ChatGPT アカウント** — 「アカウント」ページで複数の ChatGPT アカウントにログインし、OpenAI Official カードごとに使用するアカウントを選択。カードを切り替えると Codex のログインも切り替わります
- **ユニバーサルプロバイダ** — 1 つの設定を Claude Code、Codex、Gemini CLI に同期
- ワンクリック切り替え、システムトレイからのクイック切り替え（Claude Code、Codex、Gemini CLI、Grok Build）、ドラッグ＆ドロップ並び替え、インポート/エクスポート

### MCP、プロンプト & Skills

- **統一 MCP パネル** — すべての MCP サーバーを 1 か所で管理し、ツールごとにチェックを入れて同期。各ツールの既存設定からのインポート、Deep Link インポートに対応
- **プロンプト** — ツールごとに管理するプロンプトライブラリ（Markdown エディタ付き）。有効化すると、そのツールのプロンプトファイル（CLAUDE.md / AGENTS.md / GEMINI.md）に書き込み。有効化の前にファイル内の既存の内容をプロンプトライブラリへ保存するため、内容が失われることはありません。Pi では SYSTEM.md、APPEND_SYSTEM.md、プロンプトテンプレートも編集可能
- **Skills** — skills.sh を検索、または GitHub リポジトリや ZIP ファイルからワンクリックでインストール。更新の確認とワンクリックでの一括更新に対応。シンボリックリンクまたはファイルコピーで各ツールに同期し、保存場所として `~/.agents/skills` も選択可能
- 3 つのパネルはいずれも検索に対応。MCP と Skills はツールごとにワンクリックで一括有効化・一括無効化も可能

### 使用量 & コストトラッキング

- **使用量ダッシュボード** — デフォルトで各ツールのローカルセッション記録を自動スキャンし、プロバイダとモデルごとにリクエスト数、トークン、キャッシュヒット率、費用を集計。トレンドチャートとリクエスト単位のログを提供
- **クォータと残高** — プロバイダカードとトレイに、公式サブスクリプションのクォータ（Claude、ChatGPT、Gemini、SuperGrok）、Coding Plan の 5 時間 / 週 / 月のクォータ（Kimi、Zhipu GLM、MiniMax、Volcengine Ark など）、アカウント残高（DeepSeek、OpenRouter、SiliconFlow など）を直接表示。一部はプロバイダカードの「利用状況を設定」で事前に有効化が必要。その他のプロバイダではカスタム使用量スクリプトを作成可能
- **カスタム価格設定** — モデルごとに単価を設定。models.dev からのインポートも可能

### セッション管理 & ワークスペース

- **セッション管理** — 各ツールの会話履歴を閲覧・検索し、再開コマンドをコピーして会話を継続。macOS ではワンクリックでターミナルから再開可能
- **ワークスペースエディタ**（OpenClaw）— エージェントファイル（AGENTS.md、SOUL.md など）とデイリーメモリーを編集
- **メモリ**（Hermes）— Hermes の MEMORY.md と USER.md を編集

### システム & プラットフォーム

- **クラウド同期** — WebDAV（坚果云、Nextcloud、Synology NAS など）または S3 互換ストレージ（AWS S3、Cloudflare R2、Alibaba Cloud OSS、Tencent Cloud COS など）で複数のデバイス間を同期。CC Switch の設定ディレクトリを Dropbox、OneDrive、iCloud などのクラウドストレージのフォルダに置くことも可能
- **CLI ツール管理** — 「バージョン情報」ページで Claude Code、Codex などのコマンドラインツールの現在のバージョンと最新バージョンを確認し、ワンクリックでインストール、アップグレード、一括アップグレード。重複インストールの診断にも対応。Windows では WSL 内のツールも管理可能（よくある質問を参照）
- **Deep Link**（`ccslite://`）— リンクからプロバイダ、MCP サーバー、プロンプトをワンクリックでインポート、またはスキルリポジトリを追加
- **便利ツール** — Claude Code の初回確認のスキップ、AI 署名の非表示、VS Code の Claude Code 拡張を CC Switch のプロバイダ切り替えに追従させる機能など
- ダーク / ライト / システムテーマ、自動起動、自動アップデーター、アトミック書き込み、自動バックアップ、多言語対応（簡体中文/繁體中文/英/日）

## よくある質問

<details>
<summary><strong>CC Switch はどの AI ツールに対応していますか？</strong></summary>

CC Switch は **Claude Code**、**Codex**、**Gemini CLI**、**Grok Build**、**OpenCode**、**OpenClaw**、**Hermes**、**Pi**、**MiniMax Code** の 9 つのツールに対応しています。各ツールに専用のプロバイダプリセットと設定管理が用意されています。ツールごとに対応している機能は[ツール別の対応機能](#ツール別の対応機能)をご覧ください。

</details>

<details>
<summary><strong>プロバイダを切り替えた後、ターミナルの再起動は必要ですか？</strong></summary>

ツールによって異なります：

- **Claude Code**：プロバイダデータのホットスイッチに対応しており、再起動は不要です。
- **Codex、Gemini CLI、Grok Build**：変更を反映するにはターミナルまたは CLI ツールを再起動してください（切り替え後に通知が表示されます）。
- **OpenCode、OpenClaw、Hermes、Pi、MiniMax Code**：これらは共存型のツールです。「追加」（Pi では「有効化」）をクリックするとプロバイダがツール自身の設定に書き込まれ、他のプロバイダと共存します。その後、ツール内で使用するモデルを選んでください。

</details>

<details>
<summary><strong>プロバイダを切り替えると、プラグインやフックなどの設定も変わってしまいますか？</strong></summary>

変わりません。Claude Code、Codex、Gemini CLI、Grok Build でプロバイダを切り替えるとき、CC Switch が置き換えるのは設定ファイルの**主要フィールド**だけです。対象はリクエスト先アドレス、キー、モデル名、API プロトコル（Codex は推論レベル、Gemini CLI は認証方式も含む）と、プロバイダに合わせて切り替わる一部の互換オプション（Claude Code の「Artifact ツールを無効化」やコンテキストウィンドウなど）です。プラグイン、フック、権限、MCP、自分で追加した環境変数、コメント、書式はそのまま残り、すべてのプロバイダに適用されます。

これらの共有設定は、ツール内で変更しても、設定ファイルを直接編集しても構いません。CC Switch で任意のプロバイダを編集して変更することもできます。エディタには「このプロバイダに切り替えた後の設定ファイルの内容」が表示され、保存すると主要フィールドはそのプロバイダに保存され、それ以外の変更は設定ファイルに書き込まれてすべてのプロバイダに適用されます。

そのため、以前の「共通設定スニペット」は不要になり、関連するボタンは削除されました。アップグレード前にスニペットに入れていた設定は、切り替えの際にすでに設定ファイルへ書き込まれているので、そのまま残ります。また CC Switch は、各設定ファイルを初めて書き換える前に、元のファイルを `~/.ccs-lite/backups/live-first-write/` にバックアップします。

</details>

<details>
<summary><strong>ツール内でモデルを変えたのに、別のプロバイダに切り替えて戻すと元に戻ってしまうのはなぜですか？</strong></summary>

モデルは主要フィールドで、プロバイダに属します。ツール内で変えたモデル（Claude Code の `/model` など）は次に切り替えるまで有効です。切り替えると、設定ファイルのモデルは切り替え先のプロバイダに保存されたものに置き換わり、ツール内で変えたモデルが元のプロバイダに保存し直されることはありません。特定のモデルを継続して使いたい場合は、CC Switch でそのプロバイダを編集してください。

以前のバージョンは、別のプロバイダへ切り替えるときに設定ファイル全体をプロバイダに保存し直していましたが、現在はそうしていません。その方式では、プラグインなどの共有設定が 1 つのプロバイダに固定されてしまい、別のプロバイダに切り替えると失われていたためです。

</details>

<details>
<summary><strong>現在アクティブなプロバイダを削除できないのはなぜですか？</strong></summary>

CC Switch は「最小限の介入」という設計原則に従っています。アプリをアンインストールしても、各ツールは正常に動作し続けます。

そのため、同時に 1 つのプロバイダのみ有効なツール（Claude Code、Codex、Gemini CLI、Grok Build）では、すべての設定を削除すると対応するツールが使用できなくなるため、システムは常にアクティブな設定を 1 つ保持します。OpenCode、OpenClaw、Hermes、Pi、MiniMax Code などの共存型ツールにはこの制限がなく、どのプロバイダでも直接削除できます。あまり使わないツールがある場合は、設定で非表示にできます。公式ログインに戻す方法は、次の質問をご覧ください。

</details>

<details>
<summary><strong>公式ログインに戻すにはどうすればよいですか？</strong></summary>

Claude Code、Codex、Gemini CLI、Grok Build のプロバイダリストには、公式プロバイダ（**Claude Official**、**OpenAI Official**、**Google Official**、**Grok Official**）があらかじめ含まれています。削除してしまった場合は、プリセットから追加し直してください。公式プロバイダに切り替えた後、ツール自身のログインフロー（Claude Code の `/login`、Codex の `codex login` など）を実行すれば、以降は公式プロバイダとサードパーティプロバイダを自由に切り替えられます。

Codex では、CC Switch 内の「ChatGPT でログイン」から複数の ChatGPT アカウントにログインし、**OpenAI Official** カードごとに「使用するアカウント」を選べるため、複数の Plus、Pro、Team アカウントをワンクリックで切り替えられます。「Codex のログインに追従」を選んだカードは、Codex CLI 自身のログインをそのまま使用します。

</details>

<details>
<summary><strong>設定ファイルのアドレスが 127.0.0.1:15721、キーが PROXY_MANAGED になっているのはなぜですか？</strong></summary>

上流の CC Switch のローカルルーティングが書き込んだ値です。上流はローカルルーティングが有効な間、ツールの設定ファイルをローカルプロキシ（`http://127.0.0.1:15721`）に向け、キーをプレースホルダーの `PROXY_MANAGED` にし、終了時に元に戻します。上流がクラッシュしたり、強制終了されたり、元に戻す前にシャットダウンされたりすると、これらの値が残り、ツールは接続できなくなります。

ccs-lite にはローカルルーティングがありません。起動時に Claude Code、Codex、Gemini CLI、Grok Build の設定ファイルを確認し、これらの残りがあって上流のローカルプロキシが動いていない場合は、現在のプロバイダーを自動で書き戻します。上流がまだ動いていてツールを引き継いでいる場合、ccs-lite はファイルを変更せず、現在のプロバイダーのカードに通知を表示します。ccs-lite のプロバイダーを使いたい場合は、上流でローカルルーティングをオフにするか上流を終了してから、カードの「再書き込み」をクリックしてください（トレイで現在のプロバイダーをクリックしても同じです）。

</details>

<details>
<summary><strong>上流の CC Switch と ccs-lite を両方インストールしたまま使えますか？</strong></summary>

使えます。ccs-lite のデータは `~/.ccs-lite` にあります。初回起動時に、上流のデータディレクトリ（`~/.cc-switch`、または上流で設定したディレクトリ）からプロバイダー、MCP サーバー、プロンプト、Skills、ChatGPT アカウント、設定を一度だけインポートします。その後はそのディレクトリを読み書きせず、上流は自分のデータで動き続けます。ディープリンクは `ccslite://` を使い、WebDAV と S3 の同期は別の `ccs-lite` リモートディレクトリを使います。

ただし、ツールに書き込む内容は両方のアプリで共有されます。

- どちらも同じツールの設定ファイル（`~/.claude/settings.json` や `~/.codex/config.toml` など）に書き込みます。ツールがどのプロバイダーを使うかは、最後に切り替えたアプリで決まります。
- ChatGPT アカウントは同じログイン情報のままインポートされます。OpenAI はリフレッシュトークンを使うたびに新しいものに置き換えるため、一方のアプリがアカウントを更新すると、もう一方のアプリのそのアカウントは使えなくなります（Codex で現在ログインしているアカウントは除きます）。使えなくなったアプリでそのアカウントに再ログインしてください。
- ツールの skills フォルダにあるスキルは、最後に同期したアプリのコピーを指すリンクです。ccs-lite がスキルを同期すると、ツールは ccs-lite のコピー（デフォルトでは `~/.ccs-lite/skills`）を使います。
- 上流で「起動時に自動実行」がオンだった場合、ccs-lite も自分のログイン項目を登録するため、コンピューターの起動時に両方のアプリが起動します。片方だけにしたい場合は、不要なほうのアプリでこのオプションをオフにしてください。

</details>

<details>
<summary><strong>Claude Code で OpenAI 互換 API やローカルモデルを使えますか？</strong></summary>

サービスがツールに必要な形式のエンドポイントを提供している場合に限り使えます。CC Switch は設定ファイルを書き込むだけで、API 形式は変換しません。Claude Code には Anthropic Messages、Codex と Grok Build には OpenAI Responses、Gemini CLI には Gemini API のエンドポイントが必要です。多くのプロバイダ（DeepSeek、Kimi、Zhipu GLM、MiniMax など）は Anthropic 互換のエンドポイントを提供しており、対応するプリセットはすでにそれを使っています。Chat Completions しか提供していないサービスを使う場合は、形式を変換するプロキシを自分で動かし、そのアドレスをプロバイダのリクエスト先に入力してください。

</details>

<details>
<summary><strong>「接続チェック」は成功したのに、リクエストが失敗するのはなぜですか？</strong></summary>

プロバイダカードの「接続チェック」は、プロバイダのアドレスに接続できるかどうかだけを確認し、実際のモデルリクエストは送信しません。そのため、API キーやモデル名が正しいかどうかは検証できません。リクエストが失敗する場合は、キー、モデル名、そしてエンドポイントがツールに必要な形式かどうか（前の質問を参照）を確認してください。

</details>

<details>
<summary><strong>データはどこに保存されますか？</strong></summary>

デフォルトでは、すべてユーザーのホームディレクトリにある `.ccs-lite` フォルダ（Windows では `C:\Users\<ユーザー名>\.ccs-lite`）に保存されます：

- **データベース**: `cc-switch.db`（SQLite — プロバイダ、MCP、プロンプト、Skills、プロジェクト、使用量記録など）
- **ローカル設定**: `settings.json`（デバイスレベルの設定。各ツールの設定ディレクトリ、バックアップポリシー、クラウド同期の接続情報など）
- **バックアップ**: `backups/`（デフォルトでは 24 時間ごとに自動バックアップし、最新 10 件を保持。「設定 → 詳細 → バックアップと復元」で変更可能）
- **Skills**: `skills/`（設定で `~/.agents/skills` に変更可能）。デフォルトではシンボリックリンクで各ツールに同期し、失敗した場合はコピーに切り替え
- **Skill バックアップ**: `skill-backups/`（スキルのアンインストールまたは更新の前に自動作成、最新 20 件を保持）
- **ChatGPT アカウントの認証情報**: `codex_oauth_auth.json`
- **ログ**: `logs/cc-switch.log` と `crash.log`（問題を報告する際は添付してください）
- **この端末の状態**: `live-state.json`（CC Switch が各ツールの設定ファイルに前回書き込んだ内容）、`codex-login-stash.json`（サードパーティのプロバイダに切り替えたときに退避した Codex の公式ログイン。公式プロバイダに戻すと復元されます）
- **設定ファイルの原本**: `backups/live-first-write/`（CC Switch が各ツールの設定ファイルを初めて書き換える前の元のファイル）

「設定 → 詳細 → 設定ディレクトリ」で「CC Switch 設定ディレクトリ」を変更すると、`settings.json`、この端末の状態、設定ファイルの原本以外の上記ファイルはすべて新しいディレクトリに保存されるようになります。CC Switch は既存のファイルを自動では移動しないため、先に手動でコピーしておいてください。`settings.json`、この端末の状態、設定ファイルの原本はこのコンピュータ専用のもので、常にデフォルトのディレクトリに置かれ、クラウド同期の対象にもなりません。

</details>

<details>
<summary><strong>Windows で WSL 内のツールを管理するには？</strong></summary>

CC Switch は WSL を自動では認識しません。「設定 → 詳細 → 設定ディレクトリ → 設定ディレクトリの上書き（詳細）」で、対象ツールのディレクトリを WSL 内のパス（例：`\\wsl.localhost\Ubuntu\home\<ユーザー名>\.claude`）に変更して保存すると、CC Switch は WSL 内の設定を読み書きするようになります（Claude Code、Codex、Gemini CLI、Grok Build、OpenCode、OpenClaw、Hermes、Pi で設定可能）。設定後は、「バージョン情報」ページでも対応する WSL ディストリビューション内でそのツールを検出・アップグレードします。

</details>

<details>
<summary><strong>コマンドライン版やヘッドレス版はありますか？</strong></summary>

CC Switch 本体が提供しているのは、グラフィカル環境が必要なデスクトップ版のみです（システム要件は[ダウンロード & インストール](#ダウンロード--インストール)を参照）。サーバー、SSH リモート、デスクトップ環境のないマシンでは、コミュニティがメンテナンスしている **[CC Switch CLI](https://github.com/SaladDay/cc-switch-cli)** をおすすめします。対話型のターミナル UI（TUI）とコマンドラインの両方で使え、Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes、Pi に対応しています。Homebrew（`brew install cc-switch-cli`）またはインストールスクリプトでインストールできます。

CC Switch CLI は上流の CC Switch のデータディレクトリ `~/.cc-switch` とその WebDAV 同期を使います。ccs-lite のデータは `~/.ccs-lite` にあり、同期も別の `ccs-lite` リモートディレクトリと独自の形式を使うため、CLI からは ccs-lite のプロバイダーや同期データは見えません。

</details>

<details>
<summary><strong>Linux（Wayland + NVIDIA）：Web コンテンツがクリックできない・リサイズで黒画面になる</strong></summary>

AppImage は過去のネイティブ Wayland クラッシュを避けるため `GDK_BACKEND=x11`（XWayland）を強制します。新しい Wayland + NVIDIA 環境ではこれが原因で Web コンテンツ領域がクリックできなくなり（タイトルバーのボタンは動作します）、リサイズ時に黒画面になることがあります。内蔵のエスケープハッチでネイティブ Wayland に戻せます：

```bash
CC_SWITCH_GDK_BACKEND=wayland ./CC-Switch-*.AppImage
```

デスクトップアイコンから起動する場合は、`.desktop` の `Exec=` 行に追記するか（例：`env CC_SWITCH_GDK_BACKEND=wayland /path/to/AppImage`）、セッション環境で設定してください。この変数は汎用です：タイル型 Wayland コンポジタ（sway/Hyprland）でクリックが効かない場合は、逆に `CC_SWITCH_GDK_BACKEND=x11` を試してください。未設定の場合は既定の動作のままです。

</details>

その他の質問については、ユーザーマニュアルの[よくある質問](docs/user-manual/ja/5-faq/5.2-questions.md)をご覧ください。

## 貢献

Issue でのバグ報告やご提案を歓迎します！新機能を開発する前に、まず Issue を作成して実装方針をご相談ください。プロジェクトに合わない機能の PR はクローズされる場合があります。

開発環境、提出前のチェック、アーキテクチャの説明は [CONTRIBUTING.md](CONTRIBUTING.md)（英語）をご覧ください。使い方に関する質問は、まず [SUPPORT.md](SUPPORT.md) をご確認ください。セキュリティ上の脆弱性は、[SECURITY.md](SECURITY.md) に従って非公開で報告してください。

**技術スタック**：Tauri 2 · Rust · React 18 · TypeScript · SQLite

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=farion1231/cc-switch&type=Date)](https://www.star-history.com/#farion1231/cc-switch&Date)

## ライセンス

MIT © Jason Young
