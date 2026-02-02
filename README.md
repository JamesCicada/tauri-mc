# \# ⚡ Minimal Minecraft Launcher (Tauri)

#

# A \*\*lightweight, native Minecraft launcher\*\* built with \*\*Tauri (Rust backend + React frontend)\*\*.

#

# This project focuses on \*\*performance, correctness, and determinism\*\*:

# \- Near-zero idle CPU usage

# \- Extremely low memory footprint

# \- Instance-based architecture (Prism-style)

# \- Backend as the single source of truth

# \- No polling, no background tasks, no Electron

#

# This is \*\*not a prototype\*\* — Minecraft (Vanilla + Fabric) launches cleanly and reliably.

#

# ---

#

# \## ✨ Key Features

#

# \- 🧱 \*\*Instance-based system\*\*

# &nbsp; - Per-instance `.minecraft`

# &nbsp; - Persistent `instance.json`

# &nbsp; - Instances survive restarts

# \- 🚀 \*\*Vanilla \& Fabric launching\*\*

# &nbsp; - Correct JVM args

# &nbsp; - Java 21 compatible

# &nbsp; - Offline mode supported

# \- 📦 \*\*Automated downloads\*\*

# &nbsp; - Versions, libraries, assets

# &nbsp; - Fabric loader metadata

# \- 🧩 \*\*Modrinth integration\*\*

# &nbsp; - Search projects

# &nbsp; - Loader-aware mod version resolution

# &nbsp; - Deterministic installs

# \- ⚙️ \*\*Performance-first design\*\*

# &nbsp; - No polling

# &nbsp; - No background intervals

# &nbsp; - Backend-driven state only

#

# ---

#

# \## 🧠 Design Philosophy

#

# \- \*\*Backend is the source of truth\*\*

# \- \*\*Disk state > in-memory state\*\*

# \- \*\*Do nothing unless the user asks\*\*

# \- \*\*Minimal UI, minimal overhead\*\*

# \- \*\*Correctness over convenience\*\*

#

# If something is not persisted, it does not exist.

#

# ---

#

# \## 📸 Status

#

# ✅ Vanilla launch

# ✅ Fabric launch (including large modpacks)

# ✅ Instance persistence

# ✅ Modrinth mod \& modpack handling

# ⚠️ Asset rendering edge cases (known, non-blocking)

#

# ---

#

# \## 🗂️ Project Structure (Simplified)

#

# \\`\\`\\`text

# src/

# ├─ commands/ # Tauri commands (backend API)

# ├─ minecraft/ # Version / asset / launch logic

# ├─ modrinth/ # Modrinth integration \& resolution

# ├─ instances/ # Instance management

# └─ frontend/ # React UI

# \\`\\`\\`

#

# ---

#

# \## 🛠️ Main TODO List

#

# \### Core

# \- \[ ] Instance schema versioning

# \- \[ ] Atomic writes for instance state

# \- \[ ] Crash-safe launch state handling

#

# \### Loader System

# \- \[ ] Fabric installer automation

# \- \[ ] Quilt loader support

# \- \[ ] Forge / NeoForge support (later)

#

# \### Mod Management

# \- \[ ] Dependency auto-resolution (Modrinth)

# \- \[ ] Mod enable / disable

# \- \[ ] Mod update detection

# \- \[ ] Clean mod removal

#

# \### Assets \& Reliability

# \- \[ ] Fix remaining asset rendering edge cases

# \- \[ ] Asset integrity verification

# \- \[ ] Transaction-style installs

#

# \### UI / UX

# \- \[ ] Prism-style instance cards

# \- \[ ] Mod browser UI

# \- \[ ] Progress phases \& better error surfaces

#

# (See full TODO in `TODO.md`)

#

# ---

#

# \## 🤝 Contributing

#

# Contributions are \*\*welcome\*\*, but this project has \*\*strict architectural rules\*\*.

#

# \### Ground Rules

#

# \- ❌ No background polling

# \- ❌ No Electron-style patterns

# \- ❌ No frontend-owned state for persistent data

# \- ✅ Backend decides, frontend reflects

# \- ✅ Deterministic, reproducible behavior

#

# \### How to Contribute

#

# 1\. Fork the repository

# 2\. Create a feature branch

#

# &nbsp; \\`\\`\\`bash

# &nbsp; git checkout -b feature/my-feature

# &nbsp; \\`\\`\\`

#

# 3\. Keep changes focused and minimal

# 4\. Follow existing patterns (especially around instance state)

# 5\. Open a pull request with a \*\*clear explanation\*\*

#

# \### Good Contribution Areas

# \- Mod dependency resolution

# \- Loader automation

# \- UI polish (without adding overhead)

# \- Reliability \& safety improvements

# \- Documentation

#

# If unsure, \*\*open an issue first\*\*.

#

# ---

#

# \## 🚫 Non-Goals

#

# This launcher intentionally does \*\*not\*\* aim to include:

# \- Telemetry or analytics

# \- Background updaters

# \- Plugin systems

# \- Embedded browsers

# \- Accounts-as-a-service features

#

# Simplicity is a feature.

#

# ---

#

# \## 📜 License

#

# License to be decided.

# (Will likely be MIT or GPL depending on distribution goals.)

#

# ---

#

# \## 🌐 Socials / Community

#

# \- GitHub: \*\*(link here)\*\*

# \- Discord: \*\*(link here)\*\*

# \- Twitter / X: \*\*(link here)\*\*

# \- Website: \*\*(optional)\*\*

#

# ---

#

# \## 🧠 Final Note

#

# This launcher exists because modern Minecraft launchers are often:

# \- bloated

# \- inefficient

# \- state-inconsistent

#

# This project proves that a \*\*native, fast, and correct launcher is still possible\*\*.

#

# If that resonates with you — welcome aboard.

# Also you may say this readme is slop for now
