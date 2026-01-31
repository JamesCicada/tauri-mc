# Launcher config (optional)

This folder is for **launcher** configuration only (e.g. future app settings). It is **not** used by the Rust code at runtime.

**Minecraft mod configs** (Entity Culling, Sodium, Crash Assistant, etc.) belong in the **game** directory when Minecraft runs: each instance uses `{instance_dir}/.minecraft/config/`. Do not put game or mod config files here.
