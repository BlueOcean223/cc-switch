<div align="center">

# CC Switch

### Der All-in-One-Manager für Claude Code, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, Hermes Agent, Pi & MiniMax Code

**API-Anbieter mit einem Klick wechseln und MCP, Skills und Prompts zentral verwalten — ohne JSON-, TOML- oder YAML-Konfigurationsdateien von Hand zu bearbeiten.**

[![Version](https://img.shields.io/github/v/release/farion1231/cc-switch?color=blue&label=version)](https://github.com/farion1231/cc-switch/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/farion1231/cc-switch/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)
[![Downloads](https://img.shields.io/github/downloads/farion1231/cc-switch/total)](https://github.com/farion1231/cc-switch/releases/latest)

<a href="https://trendshift.io/repositories/15372" target="_blank"><img src="https://trendshift.io/api/badge/repositories/15372" alt="farion1231%2Fcc-switch | Trendshift" style="width: 250px; height: 55px;" width="250" height="55"/></a>
<a href="https://www.star-history.com/#farion1231/cc-switch&Date"><picture><source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/badge?repo=farion1231/cc-switch&theme=dark" /><img alt="Star History Rank" src="https://api.star-history.com/badge?repo=farion1231/cc-switch" width="196" height="55" /></picture></a>

Dieses Repository ist eine abgespeckte Version von [farion1231/cc-switch](https://github.com/farion1231/cc-switch), die nur Konfigurationsdateien verwaltet. Die Upstream-Website ccswitch.io sowie die Homebrew- und AUR-Pakete liefern die Upstream-Version.

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch | [Changelog](CHANGELOG.md)

**[Download](#download--installation) · [Schnellstart](#schnellstart) · [Funktionen](#funktionen) · [FAQ](#faq) · [Benutzerhandbuch (Englisch)](docs/user-manual/en/README.md)**

</div>

## Warum CC Switch?

Claude Code, Codex, Gemini CLI und andere KI-Programmierwerkzeuge haben jeweils ihr eigenes Konfigurationsformat. Wer den API-Anbieter wechselt, muss JSON-, TOML-, YAML- oder `.env`-Dateien von Hand bearbeiten; auch MCP, Skills und Prompts müssen in jedem Werkzeug einzeln gepflegt werden.

**CC Switch** bündelt all das in einer einzigen Desktop-App: Preset auswählen, Schlüssel eintragen und mit einem Klick wechseln — Ihre bestehende Konfiguration geht dabei nicht verloren.

- **Eine App, neun Werkzeuge** — Claude Code, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, Hermes, Pi, MiniMax Code
- **Kein manuelles Bearbeiten mehr** — 60+ Anbieter-Presets, darunter AWS Bedrock, NVIDIA NIM, OpenRouter, DeepSeek und Kimi
- **Nur Konfigurationsdateien** — CC Switch schreibt ausschließlich die Konfigurationsdateien der Werkzeuge; es betreibt keinen lokalen Proxy und leitet keine Anfragen weiter
- **MCP, Skills & Prompts zentral verwalten** — MCP und Skills einmal hinzufügen und pro Werkzeug per Häkchen synchronisieren; Prompts werden je Werkzeug separat gepflegt
- **Nutzung und Kontingente auf einen Blick** — Token-Verbrauch und Kosten werden aus den lokalen Sitzungsprotokollen der Werkzeuge erfasst; Abo-Kontingente und Guthaben erscheinen direkt auf den Anbieterkarten und im System-Tray
- **Plattformübergreifend** — Native Desktop-App für Windows, macOS und Linux, gebaut mit Tauri 2

## Screenshots

|                  Hauptoberfläche                   |                  Anbieter hinzufügen                  |
| :-----------------------------------------------: | :--------------------------------------------: |
| ![Hauptoberfläche](assets/screenshots/main-en.png) | ![Anbieter hinzufügen](assets/screenshots/add-en.png) |

## Download & Installation

### Systemanforderungen

- **Windows**: Windows 10 und höher
- **macOS**: macOS 12 (Monterey) und höher
- **Linux**: x86_64 oder ARM64 mit glibc 2.35+ und WebKitGTK 4.1 — z. B. Ubuntu 22.04+, Debian 12+ und aktuelle Fedora-Versionen; RHEL / Rocky / Alma 8–9 werden derzeit nicht unterstützt

### Windows-Nutzer

Laden Sie das neueste Installationsprogramm `CC-Switch-v{version}-Windows.msi` oder die portable Version `CC-Switch-v{version}-Windows-Portable.zip` von der Seite [Releases](../../releases) herunter. Unter Windows on ARM laden Sie `CC-Switch-v{version}-Windows-arm64.msi` oder `CC-Switch-v{version}-Windows-arm64-Portable.zip` herunter.

### macOS-Nutzer

Laden Sie `CC-Switch-v{version}-macOS.dmg` (empfohlen) oder `.zip` von der Seite [Releases](../../releases) herunter. Es handelt sich um einen Universal-Build, der nativ auf Apple-Silicon- und Intel-Macs läuft.

> **Hinweis**: Ob das macOS-Paket von Apple notarisiert ist, steht in den Release Notes der jeweiligen Version. Eine nicht notarisierte Version blockiert macOS beim ersten Öffnen; führen Sie im Terminal `xattr -dr com.apple.quarantine "/Applications/ccs-lite.app"` aus und öffnen Sie die App erneut.

### Linux-Nutzer

Laden Sie den neuesten Linux-Build von der Seite [Releases](../../releases) herunter:

- `CC-Switch-v{version}-Linux-x86_64.deb` / `-Linux-arm64.deb` (Debian/Ubuntu)
- `CC-Switch-v{version}-Linux-x86_64.rpm` / `-Linux-arm64.rpm` (Fedora und andere RPM-Distributionen mit WebKitGTK 4.1)
- `CC-Switch-v{version}-Linux-x86_64.AppImage` / `-Linux-arm64.AppImage` (jede Distribution, die die obigen Systemanforderungen erfüllt)

> **Flatpak**: Nicht in den offiziellen Releases enthalten. Sie können es selbst aus dem `.deb` bauen — eine Anleitung finden Sie unter [`flatpak/README.md`](flatpak/README.md).

## Schnellstart

### Grundlegende Verwendung

1. **Anbieter hinzufügen**: Klicken Sie in der Symbolleiste auf „Add New Provider“ (die +-Schaltfläche) → Wählen Sie ein Preset oder erstellen Sie eine eigene Konfiguration
2. **Anbieter wechseln**:
   - Hauptoberfläche: Anbieter auswählen → auf „Enable“ klicken (bei OpenCode, OpenClaw, Hermes und MiniMax Code heißt die Schaltfläche „Add“; diese vier Werkzeuge und Pi arbeiten im Parallelmodus, sodass Sie mehrere Anbieter gleichzeitig hinzufügen können)
   - System-Tray: Anbietername direkt anklicken (Claude Code, Codex, Gemini CLI, Grok Build)
3. **Wirksam werden**: Claude Code erfordert keinen Neustart; bei Codex, Gemini CLI und Grok Build starten Sie das Terminal oder das CLI-Werkzeug neu (siehe FAQ)
4. **Zurück zum offiziellen Login**: Wechseln Sie zum mitgelieferten offiziellen Anbieter in der Liste (z. B. „Claude Official“), starten Sie das Werkzeug neu und folgen Sie dann seinem Login-/OAuth-Vorgang

### MCP, Prompts, Skills, Projekte & Sessions

- **MCP**: Klicken Sie auf „MCP Management“ → Server über Vorlagen oder eigene Konfiguration hinzufügen (oder „Import Existing“) → Synchronisierung pro Werkzeug umschalten
- **Prompts**: Klicken Sie auf „Prompts“ → Prompts mit dem Markdown-Editor erstellen → nach dem Aktivieren wird der Prompt in die Prompt-Datei des Werkzeugs geschrieben
- **Skills**: Klicken Sie auf „Skills“ → „Discover Skills“ → skills.sh durchsuchen oder GitHub-Repositorys durchstöbern → mit einem Klick in unterstützte Werkzeuge installieren
- **Projekte**: Öffnen Sie auf der Seite von Claude Code oder Codex den Projektumschalter oben auf der Hauptseite → „New project“, um die aktuelle Konfiguration zu speichern; später wählen Sie das Projekt einfach im Umschalter aus, um die gesamte Konfiguration auf einmal zu wechseln
- **Sessions**: Klicken Sie auf „Session Manager“ → Gesprächsverlauf der einzelnen Werkzeuge durchsehen, durchsuchen und fortsetzen

> **Hinweis**: Beim Erststart importiert CC Switch Ihre bestehende Konfiguration von Claude Code, Codex, Gemini CLI und Grok Build automatisch als Anbieter namens `default` und fügt für diese Werkzeuge jeweils den offiziellen Anbieter hinzu, sodass nichts von Ihrer bisherigen Konfiguration verloren geht.

Ausführliche Anleitungen zu allen Funktionen finden Sie im **[Benutzerhandbuch](docs/user-manual/en/README.md)** (auf Englisch) — es deckt sämtliche Funktionen ab, darunter Anbieterverwaltung, MCP/Prompts/Skills sowie die Nutzungsstatistik.

## Funktionen

[Vollständiges Changelog](CHANGELOG.md) | [Release Notes](docs/release-notes/v3.20.4-en.md)

### Funktionen je Werkzeug

| Werkzeug | Anbieter | Tray-Umschaltung | MCP | Skills | Prompts | Sessions | Nutzungsstatistik |
| --- | --- | :---: | :---: | :---: | --- | :---: | :---: |
| Claude Code | Wechsel | ✓ | ✓ | ✓ | CLAUDE.md | ✓ | ✓ |
| Codex | Wechsel | ✓ | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| Gemini CLI | Wechsel | ✓ | ✓ | ✓ | GEMINI.md | ✓ | ✓ |
| Grok Build | Wechsel | ✓ | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| OpenCode | Parallel | – | ✓ | ✓ | AGENTS.md | ✓ | ✓ |
| OpenClaw | Parallel | – | – | – | Workspace-Editor | ✓ | – |
| Hermes | Parallel | – | ✓ | ✓ | Memory-Verwaltung | ✓ | – |
| Pi | Parallel | – | – | ✓ | AGENTS.md, SYSTEM.md, Prompt-Vorlagen | ✓ | ✓ |
| MiniMax Code | Parallel | – | ✓ | ✓ | AGENTS.md | ✓ | ✓ |

- **Wechsel**: Es ist jeweils nur ein Anbieter aktiv; **Parallel**: Mehrere Anbieter werden gleichzeitig in die eigene Konfiguration des Werkzeugs geschrieben; welcher verwendet wird, wählen Sie im Werkzeug aus.
- **Sessions**: Sitzungsverlauf durchsehen und durchsuchen, Befehl zum Fortsetzen kopieren und das Gespräch weiterführen (Sessions von OpenClaw und Hermes lassen sich derzeit nicht fortsetzen). Um Hermes-Sessions zu sehen, wählen Sie im Session Manager „All“.
- **Nutzungsstatistik**: Sie wird aus den lokalen Sitzungsprotokollen der einzelnen Werkzeuge erstellt.

### Anbieterverwaltung

- **60+ Anbieter-Presets** — Preset auswählen und Schlüssel eintragen, um einen Anbieter hinzuzufügen; alternativ können Sie eine eigene Konfiguration erstellen
- **Nur Kernfelder** — Beim Wechsel werden nur die Verbindungsdaten wie Endpunkt, Schlüssel und Modell ersetzt; Plugins, Hooks, MCP, selbst hinzugefügte Einstellungen und Kommentare bleiben unverändert
- **Projekte** — Speichern Sie den aktuellen Anbieter sowie MCP, Skills und Prompt-Dateien von Claude Code oder Codex als Projekt und wechseln Sie später über den Projektumschalter oben auf der Hauptseite oder über das System-Tray mit einem Klick die gesamte Konfiguration; beim Wechsel zu einem anderen Projekt wird der aktuelle Zustand automatisch im bisherigen Projekt gespeichert
- **Mehrere ChatGPT-Konten für Codex** — Melden Sie sich auf der Seite „Accounts“ bei mehreren ChatGPT-Konten an und wählen Sie für jede OpenAI-Official-Karte das zu verwendende Konto; mit dem Kartenwechsel wechselt auch der Codex-Login
- **Universelle Anbieter** — Eine Konfiguration synchronisiert sich mit Claude Code, Codex und Gemini CLI
- Umschaltung mit einem Klick, Schnellumschaltung über System-Tray (Claude Code, Codex, Gemini CLI, Grok Build), Sortierung per Drag-and-drop, Import/Export

### MCP, Prompts & Skills

- **Einheitliches MCP-Panel** — Alle MCP-Server an einer Stelle verwalten, pro Werkzeug per Häkchen synchronisieren, bestehende Konfigurationen aus den einzelnen Werkzeugen importieren, Import per Deep Link
- **Prompts** — Je Werkzeug separat verwaltete Prompt-Bibliothek mit Markdown-Editor; nach dem Aktivieren wird der Prompt in die Prompt-Datei des Werkzeugs geschrieben (CLAUDE.md / AGENTS.md / GEMINI.md), wobei der bisherige Inhalt der Datei vorher in die Prompt-Bibliothek zurückgesichert wird und nicht verloren geht. Bei Pi lassen sich außerdem SYSTEM.md, APPEND_SYSTEM.md und Prompt-Vorlagen bearbeiten
- **Skills** — skills.sh durchsuchen oder mit einem Klick aus GitHub-Repositorys bzw. ZIP-Dateien installieren; nach Updates suchen und alle mit einem Klick aktualisieren; Synchronisierung in die einzelnen Werkzeuge per Symlink oder Dateikopie, als Speicherort ist optional `~/.agents/skills` wählbar
- Alle drei Panels unterstützen die Suche; bei MCP und Skills können Sie zudem pro Werkzeug alle Einträge mit einem Klick aktivieren oder deaktivieren

### Nutzungs- & Kostenverfolgung

- **Nutzungs-Dashboard** — Standardmäßig werden die lokalen Sitzungsprotokolle der einzelnen Werkzeuge automatisch gescannt und Anfragen, Token, Cache-Trefferquote und Kosten nach Anbieter und Modell ausgewertet — mit Trenddiagrammen und einem Protokoll jeder einzelnen Anfrage
- **Kontingente & Guthaben** — Anbieterkarten und System-Tray zeigen direkt offizielle Abo-Kontingente (Claude, ChatGPT, Gemini, SuperGrok), 5-Stunden-, Wochen- und Monatskontingente von Coding Plans (Kimi, Zhipu GLM, MiniMax, Volcengine Ark u. a.) sowie Kontoguthaben (DeepSeek, OpenRouter, SiliconFlow u. a.) an; manche davon müssen zuerst über „Configure usage query“ auf der Anbieterkarte aktiviert werden. Für andere Anbieter können Sie ein eigenes Nutzungsskript schreiben
- **Eigene Preise** — Preise pro Modell festlegen, Import von models.dev möglich

### Session Manager & Workspace

- **Session Manager** — Gesprächsverlauf der einzelnen Werkzeuge durchsehen und durchsuchen, Befehl zum Fortsetzen kopieren und das Gespräch weiterführen; unter macOS lässt sich eine Session mit einem Klick im Terminal fortsetzen
- **Workspace-Editor** (OpenClaw) — Agent-Dateien (AGENTS.md, SOUL.md usw.) und „Daily Memory“ bearbeiten
- **Memory-Verwaltung** (Hermes) — MEMORY.md und USER.md von Hermes bearbeiten

### System & Plattform

- **Cloud-Synchronisierung** — Geräteübergreifende Synchronisierung über WebDAV (Jianguoyun, Nextcloud, Synology NAS usw.) oder S3-kompatiblen Speicher (AWS S3, Cloudflare R2, Alibaba Cloud OSS, Tencent Cloud COS usw.); alternativ können Sie das CC-Switch-Konfigurationsverzeichnis in einen Cloud-Speicher-Ordner wie Dropbox, OneDrive oder iCloud legen
- **CLI-Werkzeugverwaltung** — Auf der Seite „About“ sehen Sie die installierte und die neueste Version von Kommandozeilenwerkzeugen wie Claude Code und Codex, können sie mit einem Klick installieren, aktualisieren oder alle auf einmal aktualisieren und doppelte Installationen diagnostizieren; unter Windows lassen sich auch Werkzeuge in WSL verwalten (siehe FAQ)
- **Deep Link** (`ccswitch://`) — Anbieter, MCP-Server und Prompts per Link mit einem Klick importieren oder Skill-Repositorys hinzufügen
- **Hilfsprogramme** — Überspringen der Erststart-Bestätigung von Claude Code, Ausblenden der KI-Attribution, Übernahme des in CC Switch gewählten Anbieters durch die Claude-Code-Erweiterung für VS Code und mehr
- Dunkles / Helles / System-Theme, automatischer Start, automatischer Updater, atomare Schreibvorgänge, automatische Backups, i18n (zh/zh-TW/en/ja)

## FAQ

<details>
<summary><strong>Welche KI-Werkzeuge unterstützt CC Switch?</strong></summary>

CC Switch unterstützt neun Werkzeuge: **Claude Code**, **Codex**, **Gemini CLI**, **Grok Build**, **OpenCode**, **OpenClaw**, **Hermes**, **Pi**, **MiniMax Code**. Jedes Werkzeug verfügt über dedizierte Anbieter-Presets und Konfigurationsverwaltung; welche Funktionen jeweils unterstützt werden, sehen Sie unter [Funktionen je Werkzeug](#funktionen-je-werkzeug).

</details>

<details>
<summary><strong>Muss ich das Terminal nach einem Anbieterwechsel neu starten?</strong></summary>

Das hängt vom Werkzeug ab:

- **Claude Code**: unterstützt Hot-Switching von Anbieterdaten — kein Neustart nötig.
- **Codex, Gemini CLI, Grok Build**: Starten Sie Ihr Terminal oder das CLI-Werkzeug neu, damit die Änderungen wirksam werden (CC Switch erinnert Sie nach dem Wechsel daran).
- **OpenCode, OpenClaw, Hermes, Pi, MiniMax Code**: Dies sind Werkzeuge im Parallelmodus — ein Klick auf „Add“ (bei Pi „Enable“) trägt den Anbieter zusätzlich zu den bereits vorhandenen in die eigene Konfiguration des Werkzeugs ein; das gewünschte Modell wählen Sie anschließend im Werkzeug aus.

</details>

<details>
<summary><strong>Ändert ein Anbieterwechsel meine Plugins, Hooks oder andere Einstellungen?</strong></summary>

Nein. Beim Anbieterwechsel für Claude Code, Codex, Gemini CLI oder Grok Build ersetzt CC Switch in der Konfigurationsdatei nur die **Kernfelder**: Endpunkt, Schlüssel, Modellname und API-Protokoll (bei Codex zusätzlich die Reasoning-Stufe, bei Gemini CLI die Authentifizierungsmethode) sowie einige Kompatibilitätsoptionen, die zum Anbieter gehören (etwa „Disable Artifact Tool“ bei Claude Code und das Kontextfenster). Plugins, Hooks, Berechtigungen, MCP, selbst hinzugefügte Umgebungsvariablen, Kommentare und Formatierung bleiben unverändert und gelten für alle Anbieter.

Diese gemeinsamen Einstellungen können Sie direkt im Werkzeug ändern oder die Konfigurationsdatei von Hand bearbeiten. Sie können auch einen beliebigen Anbieter in CC Switch bearbeiten: Der Editor zeigt, „wie die Konfigurationsdatei nach dem Wechsel zu diesem Anbieter aussieht“. Beim Speichern werden die Kernfelder in diesem Anbieter gespeichert; alle anderen Änderungen werden in die Konfigurationsdatei geschrieben und gelten für alle Anbieter.

Das frühere „Common Config Snippet“ wird daher nicht mehr gebraucht, und die zugehörigen Schaltflächen wurden entfernt. Einstellungen, die vor dem Upgrade im Snippet standen, wurden beim Wechseln bereits in die Konfigurationsdatei geschrieben und bleiben dort erhalten. Bevor CC Switch eine Konfigurationsdatei zum ersten Mal umschreibt, sichert es außerdem das Original unter `~/.cc-switch/backups/live-first-write/`.

</details>

<details>
<summary><strong>Ich habe im Werkzeug das Modell gewechselt — warum ist es nach dem Hin- und Zurückwechseln wieder das alte?</strong></summary>

Das Modell ist ein Kernfeld und gehört zum Anbieter. Ein im Werkzeug gewähltes Modell (etwa mit `/model` in Claude Code) gilt bis zum nächsten Wechsel; beim Wechsel wird das Modell in der Konfigurationsdatei durch das im Zielanbieter gespeicherte ersetzt, und CC Switch speichert das im Werkzeug gewählte Modell nicht im vorherigen Anbieter. Wenn Sie ein Modell dauerhaft nutzen möchten, bearbeiten Sie diesen Anbieter in CC Switch.

Ältere Versionen haben beim Wegwechseln die gesamte Konfigurationsdatei in den Anbieter zurückgeschrieben. Das passiert nicht mehr: Dadurch wurden Plugins und andere gemeinsame Einstellungen in einem einzelnen Anbieter eingefroren und gingen beim Wechsel zu einem anderen Anbieter verloren.

</details>

<details>
<summary><strong>Warum kann ich den aktuell aktiven Anbieter nicht löschen?</strong></summary>

CC Switch folgt dem Designprinzip der „minimalen Eingriffstiefe“ — selbst wenn Sie die App deinstallieren, funktionieren Ihre Werkzeuge weiterhin normal.

Bei Werkzeugen mit jeweils einem aktiven Anbieter (Claude Code, Codex, Gemini CLI, Grok Build) behält das System daher immer eine aktive Konfiguration bei, da das Löschen aller Konfigurationen das entsprechende Werkzeug unbrauchbar machen würde. Werkzeuge im Parallelmodus wie OpenCode, OpenClaw, Hermes, Pi und MiniMax Code sind davon nicht betroffen — dort können Sie jeden Anbieter direkt löschen. Wenn Sie ein Werkzeug selten verwenden, können Sie es in den Einstellungen ausblenden. Wie Sie zurück zum offiziellen Login wechseln, erfahren Sie in der nächsten Frage.

</details>

<details>
<summary><strong>Wie wechsle ich zurück zum offiziellen Login?</strong></summary>

In CC Switch enthält die Anbieterliste von Claude Code, Codex, Gemini CLI und Grok Build jeweils bereits einen offiziellen Anbieter (**Claude Official**, **OpenAI Official**, **Google Official**, **Grok Official**); falls Sie ihn gelöscht haben, fügen Sie ihn aus den Presets wieder hinzu. Folgen Sie nach dem Wechsel zum offiziellen Anbieter dem Login-Vorgang des Werkzeugs (z. B. `/login` in Claude Code, `codex login` für Codex); anschließend können Sie frei zwischen dem offiziellen Anbieter und Drittanbietern wechseln.

Codex kann sich in CC Switch außerdem über „Sign in with ChatGPT“ bei mehreren ChatGPT-Konten anmelden; für jede **OpenAI Official**-Karte wählen Sie dann unter „Account to use“ ein Konto aus, sodass der Wechsel zwischen mehreren Plus-, Pro- oder Team-Konten mit einem Klick gelingt. Karten mit „Follow Codex login“ verwenden weiterhin den eigenen Login der Codex CLI.

</details>

<details>
<summary><strong>Warum zeigt meine Konfigurationsdatei auf 127.0.0.1:15721 mit dem Schlüssel PROXY_MANAGED?</strong></summary>

Diese Werte hat das lokale Routing des Upstream-CC-Switch geschrieben. Solange das lokale Routing aktiv ist, lässt Upstream die Konfigurationsdatei des Werkzeugs auf seinen lokalen Proxy (`http://127.0.0.1:15721`) zeigen, mit dem Platzhalterschlüssel `PROXY_MANAGED`, und stellt die Datei beim Beenden wieder her. Stürzt Upstream ab, wird es beendet oder fährt der Rechner herunter, bevor die Datei wiederhergestellt ist, bleiben die Werte stehen und das Werkzeug kann keine Verbindung aufbauen.

ccs-lite hat kein lokales Routing. Beim Start prüft es die Konfigurationsdateien von Claude Code, Codex, Gemini CLI und Grok Build: Findet es diese Reste und läuft der lokale Proxy von Upstream nicht, schreibt es den aktuellen Anbieter automatisch zurück. Läuft Upstream noch und routet das Werkzeug, lässt ccs-lite die Datei unverändert und zeigt einen Hinweis auf der Karte des aktuellen Anbieters. Um stattdessen den Anbieter aus ccs-lite zu verwenden, schalten Sie das lokale Routing in Upstream aus oder beenden Upstream und klicken dann auf der Karte auf „Neu schreiben“ (oder klicken Sie im Tray auf den aktuellen Anbieter).

</details>

<details>
<summary><strong>Kann ich in Claude Code OpenAI-kompatible Schnittstellen oder lokale Modelle verwenden?</strong></summary>

Nur wenn der Dienst einen Endpunkt in dem Format anbietet, das das Werkzeug erwartet. CC Switch schreibt nur Konfigurationsdateien und konvertiert keine Schnittstellenformate: Claude Code braucht einen Anthropic-Messages-Endpunkt, Codex und Grok Build einen OpenAI-Responses-Endpunkt und Gemini CLI die Gemini-API. Viele Anbieter (DeepSeek, Kimi, Zhipu GLM, MiniMax u. a.) bieten einen Anthropic-kompatiblen Endpunkt an, und ihre Presets verwenden ihn bereits. Für einen Dienst, der nur Chat Completions anbietet, betreiben Sie selbst einen Proxy zur Formatkonvertierung und tragen dessen Adresse als Endpunkt des Anbieters ein.

</details>

<details>
<summary><strong>Der „Connectivity check“ war erfolgreich — warum schlagen Anfragen trotzdem fehl?</strong></summary>

Der „Connectivity check“ auf der Anbieterkarte prüft nur, ob die Anbieteradresse erreichbar ist, und sendet keine echte Modellanfrage; ob API-Schlüssel und Modellname korrekt sind, lässt sich damit also nicht überprüfen. Wenn Anfragen fehlschlagen, prüfen Sie Schlüssel, Modellname und ob der Endpunkt das Format verwendet, das das Werkzeug erwartet (siehe vorherige Frage).

</details>

<details>
<summary><strong>Wo werden meine Daten gespeichert?</strong></summary>

Standardmäßig liegen alle Daten im Ordner `.cc-switch` in Ihrem Benutzerverzeichnis (unter Windows `C:\Users\<Benutzername>\.cc-switch`):

- **Datenbank**: `cc-switch.db` (SQLite — Anbieter, MCP, Prompts, Skills, Projekte, Nutzungsdaten usw.)
- **Lokale Einstellungen**: `settings.json` (gerätebezogene Einstellungen, z. B. die Konfigurationsverzeichnisse der einzelnen Werkzeuge, Backup-Richtlinie, Verbindungsdaten für die Cloud-Synchronisierung)
- **Backups**: `backups/` (standardmäßig automatisch alle 24 Stunden, die 10 neuesten werden behalten; anpassbar unter „Settings → Advanced → Backup & Restore“)
- **Skills**: `skills/` (in den Einstellungen auf `~/.agents/skills` umstellbar); standardmäßig per Symlink mit den einzelnen Werkzeugen synchronisiert; schlägt das fehl, wird stattdessen kopiert
- **Skill-Backups**: `skill-backups/` (vor dem Deinstallieren oder Aktualisieren eines Skills automatisch erstellt, die 20 neuesten werden behalten)
- **ChatGPT-Anmeldedaten**: `codex_oauth_auth.json`
- **Logs**: `logs/cc-switch.log` und `crash.log` — bitte fügen Sie sie bei Problemmeldungen bei
- **Gerätezustand**: `live-state.json` (was CC Switch zuletzt in die Konfigurationsdateien der Werkzeuge geschrieben hat), `codex-login-stash.json` (der offizielle Codex-Login, der beim Wechsel zu einem Drittanbieter beiseitegelegt und beim Zurückwechseln zu einem offiziellen Anbieter wiederhergestellt wird)
- **Original-Konfigurationsdateien**: `backups/live-first-write/` (die Konfigurationsdateien der Werkzeuge, bevor CC Switch sie zum ersten Mal umgeschrieben hat)

Wenn Sie unter „Settings → Advanced → Configuration Directory“ das „CC Switch Configuration Directory“ ändern, werden alle oben genannten Dateien außer `settings.json`, dem Gerätezustand und den Original-Konfigurationsdateien im neuen Verzeichnis abgelegt. CC Switch verschiebt vorhandene Dateien nicht automatisch; kopieren Sie sie vorher manuell dorthin. `settings.json`, der Gerätezustand und die Original-Konfigurationsdateien gehören nur zu diesem Rechner: Sie bleiben immer im Standardverzeichnis und werden nicht per Cloud synchronisiert.

</details>

<details>
<summary><strong>Wie verwalte ich unter Windows Werkzeuge in WSL?</strong></summary>

CC Switch erkennt WSL nicht automatisch. Ändern Sie unter „Settings → Advanced → Configuration Directory → Configuration Directory Override (Advanced)“ das Verzeichnis des jeweiligen Werkzeugs auf einen Pfad in WSL, z. B. `\\wsl.localhost\Ubuntu\home\<Benutzername>\.claude`; nach dem Speichern liest und schreibt CC Switch die Konfiguration in WSL (einstellbar für Claude Code, Codex, Gemini CLI, Grok Build, OpenCode, OpenClaw, Hermes und Pi). Anschließend erkennt und aktualisiert auch die Seite „About“ das Werkzeug in der entsprechenden WSL-Distribution.

</details>

<details>
<summary><strong>Gibt es eine Kommandozeilen- oder Headless-Version?</strong></summary>

CC Switch selbst gibt es nur als Desktop-Version mit grafischer Oberfläche (Systemanforderungen siehe [Download & Installation](#download--installation)). Für Server, SSH-Remote-Sitzungen oder Rechner ohne Desktop-Umgebung empfehlen wir das von der Community gepflegte **[CC Switch CLI](https://github.com/SaladDay/cc-switch-cli)**: Es bietet sowohl eine interaktive Terminaloberfläche (TUI) als auch eine Kommandozeilenschnittstelle, unterstützt Claude Code, Codex, Gemini CLI, OpenCode, OpenClaw, Hermes und Pi und lässt sich über Homebrew (`brew install cc-switch-cli`) oder ein Installationsskript installieren.

CC Switch CLI verwendet standardmäßig dasselbe Datenverzeichnis `~/.cc-switch` wie die Desktop-Version und ist auch mit deren WebDAV-Synchronisierung kompatibel. Die beiden Projekte werden unabhängig voneinander veröffentlicht, daher kann die von der CLI-Version unterstützte Datenbankversion zeitweise hinter der Desktop-Version zurückliegen; erscheint der Hinweis, dass die Datenbankversion zu neu ist, aktualisieren Sie die CLI-Version oder warten Sie, bis sie nachzieht.

</details>

<details>
<summary><strong>Linux (Wayland + NVIDIA): Klicks im Webinhalt reagieren nicht, schwarzer Bildschirm beim Größenändern</strong></summary>

Das AppImage erzwingt `GDK_BACKEND=x11` (XWayland), um einen historischen nativen Wayland-Absturz zu vermeiden. Auf neueren Wayland-+-NVIDIA-Systemen kann das dazu führen, dass der Webinhalt nicht anklickbar ist (die Titelleisten-Schaltflächen funktionieren weiterhin) und das Fenster beim Größenändern schwarz wird. Starten Sie mit dem optionalen Notausgang, um zu nativem Wayland zu wechseln:

```bash
CC_SWITCH_GDK_BACKEND=wayland ./CC-Switch-*.AppImage
```

Wenn Sie über ein Desktop-Symbol starten, fügen Sie es der `Exec=`-Zeile der `.desktop`-Datei hinzu (z. B. `env CC_SWITCH_GDK_BACKEND=wayland /pfad/zum/AppImage`) oder setzen Sie es in Ihrer Sitzungsumgebung. Die Variable ist generisch: Auf Tiling-Wayland-Compositors (sway/Hyprland), bei denen Klicks nicht reagieren, versuchen Sie umgekehrt `CC_SWITCH_GDK_BACKEND=x11`. Bleibt sie ungesetzt, bleibt das Standardverhalten erhalten.

</details>

Weitere Fragen und Antworten finden Sie in den [FAQ des Benutzerhandbuchs](docs/user-manual/en/5-faq/5.2-questions.md) (auf Englisch).

## Mitwirken

Wir freuen uns über Issues mit Fehlerberichten und Vorschlägen! Bitte eröffnen Sie vor der Entwicklung einer neuen Funktion zunächst ein Issue, um die Umsetzung zu besprechen; Feature-PRs, die nicht zum Projekt passen, können geschlossen werden.

Entwicklungsumgebung, Prüfungen vor dem Einreichen und Architekturbeschreibung finden Sie in [CONTRIBUTING.md](CONTRIBUTING.md) (auf Englisch); bei Fragen zur Nutzung lesen Sie bitte zuerst [SUPPORT.md](SUPPORT.md); Sicherheitslücken melden Sie bitte vertraulich gemäß [SECURITY.md](SECURITY.md).

**Tech-Stack**: Tauri 2 · Rust · React 18 · TypeScript · SQLite

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=farion1231/cc-switch&type=Date)](https://www.star-history.com/#farion1231/cc-switch&Date)

## Lizenz

MIT © Jason Young
