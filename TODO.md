# Minecraft Launcher – Feature TODO

## Core (Must Be Rock-Solid)

### Instance System

- [x] Per-instance `.minecraft`
- [x] Persistent `instance.json`
- [x] Instance survives restart
- [x] Backend = source of truth
- [ ] Instance schema versioning (`schemaVersion: 1`)
- [ ] Atomic instance writes (temp → rename)

### Launching

- [x] Vanilla launch
- [x] Fabric launch
- [x] Correct JVM args
- [x] Java 21 compatibility
- [x] Offline mode
- [x] **Graceful crash detection** ⭐
- [x] **Capture & persist last launch log** ⭐
- [x] **Running / Crashed / Exited final states** ⭐

---

## Loader System

### Fabric / Quilt

- [x] Fabric metadata fetch
- [x] Fabric classpath resolution
- [x] Fabric main class fix
- [x] Large modpack compatibility
- [x] Fabric installer automation
- [ ] Quilt loader support

### Forge / NeoForge

- [ ] Loader metadata fetch
- [ ] Derived version generation
- [ ] Forge-specific JVM args
- [ ] NeoForge detection & mapping

---

## Version & Asset Management

- [x] Version manifest fetch & cache
- [x] Version JSON download
- [x] Client JAR download
- [x] Library resolution
- [x] Asset index download
- [x] Asset objects download
- [x] Fix asset rendering edge cases
- [ ] Asset integrity verification (hash check)
- [ ] Shared read-only asset cache (optional)

---

## Mod Management

### Modrinth Integration

- [x] Project search
- [x] Version fetch
- [x] Loader-aware version resolution
- [ ] Auto-pick best compatible version
- [ ] Dependency auto-resolution
- [ ] Version pinning
- [x] **Mod update detection** ⭐
- [x] **Incompatibility detection & warnings** ⭐
- [x] Mod removal (clean uninstall)

### Mods Folder

- [x] List installed mods per instance
- [x] **Read mod metadata from JAR files** ⭐
- [x] **Enable / disable mods (`.disabled`)** ⭐
- [x] **Detect broken mods (wrong loader / MC)** ⭐

---

## Modpacks

### Modrinth (.mrpack)

- [x] Parse `modrinth.index.json`
- [x] Download mods
- [x] Apply overrides
- [x] Per-instance install
- [ ] Validate loader compatibility
- [ ] Validate MC version mismatch
- [ ] Reject unsafe legacy formats
- [ ] Progress reporting per phase

---

## Authentication (Optional)

- [x] Offline mode
- [ ] Microsoft device code login
- [ ] Token refresh on launch
- [ ] Secure token storage (OS keychain)
- [ ] Skin & cape support
- [ ] Account switching per instance

---

## UI / UX

### Instance View

- [x] **Polished instance cards with better visual hierarchy** ⭐
- [x] **Loader + version badges with improved styling** ⭐
- [x] **Last played timestamp display** ⭐
- [x] **Playtime tracking & display** ⭐
- [x] **Crash indicators on instance cards** ⭐
- [ ] Context menu for quick actions
- [ ] Drag & reorder (optional)

### Mod Browser

- [x] **Enhanced mod management UI** ⭐
- [x] **Mod update detection with indicators** ⭐
- [x] **Enable/disable mods toggle** ⭐
- [x] **One-click mod updates** ⭐
- [ ] Compatible-only results filtering
- [ ] Version dropdown with update indicators

### Crash Logs & Diagnostics

- [x] **Crash log viewer with syntax highlighting** ⭐
- [x] **Last launch log persistence** ⭐
- [x] **Copy crash info to clipboard** ⭐
- [x] **Crash categorization (OOM, mod conflict, etc.)** ⭐

### Feedback

- [x] **Improved progress bars with phase indicators** ⭐
- [x] **Better error surfaces with actionable messages** ⭐
- [x] **Toast notifications for operations** ⭐
- [ ] Copy debug info button

---

## Performance Guarantees

- [x] No polling
- [x] No background intervals
- [x] No idle CPU usage
- [x] Minimal RAM footprint
- [ ] No filesystem watchers
- [ ] No React global state abuse
- [ ] Backend-driven refresh only

---

## Reliability & Safety

- [ ] Transaction-style installs
- [ ] Resume partial downloads
- [ ] Detect corrupted libraries
- [ ] Disk space preflight check
- [ ] Graceful cancellation

---

## Configuration & Settings

- [x] Java path selection
- [ ] Per-instance JVM args
- [ ] Global memory presets
- [ ] Per-instance memory override
- [ ] Environment variable injection

---

## Export / Import

- [ ] Export instance
- [ ] Import instance
- [ ] Clone instance
- [ ] Duplicate with new MC version

---

## Cleanup & Maintenance

- [x] **Remove unused game versions** ⭐
- [x] **Remove orphaned libraries** ⭐
- [x] **Clear asset cache** ⭐
- [x] **Disk usage overview with cleanup options** ⭐
- [x] **Settings panel for cleanup operations** ⭐

---

## 🎯 COMPLETED SPRINT - POLISH & FEATURES ✅

### ✅ Priority 1: Mod Management Polish

- [x] **Mod update detection & notifications**
- [x] **Incompatibility warnings for loader/MC version mismatches**
- [x] **Enable/disable mods without deletion**
- [x] **Enhanced mod browser with better search & filtering**

### ✅ Priority 2: Crash Logs & Diagnostics

- [x] **Crash detection & log persistence**
- [x] **Crash log viewer UI with copy functionality**
- [x] **Launch state tracking (Running/Crashed/Exited)**
- [x] **Error categorization & user-friendly messages**

### ✅ Priority 3: Cleanup & Maintenance

- [x] **Settings panel with cleanup options**
- [x] **Remove unused game versions not used by any instance**
- [x] **Disk usage overview**
- [x] **Clear orphaned files & cache**

### ✅ Priority 4: UI Polish

- [x] **Refreshed instance cards with better visual hierarchy**
- [x] **Improved typography & spacing**
- [x] **Better progress indicators & loading states**
- [x] **Enhanced color scheme & consistency**
- [x] **Last played timestamps & playtime tracking**

---

## 🚀 NEXT SPRINT - ADVANCED FEATURES

### Priority 1: Dependency Management

- [ ] **Auto-resolve mod dependencies**
- [ ] **Dependency conflict detection**
- [ ] **Bulk mod updates**
- [ ] **Mod compatibility matrix**

### Priority 2: Performance & Reliability

- [ ] **Transaction-style installs with rollback**
- [ ] **Resume partial downloads**
- [ ] **Asset integrity verification**
- [ ] **Disk space preflight checks**

### Priority 3: Advanced Instance Management

- [ ] **Instance cloning & duplication**
- [ ] **Export/import instances**
- [ ] **Instance templates**
- [ ] **Bulk instance operations**

### Priority 4: User Experience

- [ ] **Context menus for quick actions**
- [ ] **Drag & drop instance reordering**
- [ ] **Advanced search & filtering**
- [ ] **Keyboard shortcuts**

---

## Release Prep

- [ ] Stable instance schema v1
- [ ] Crash-safe writes everywhere
- [ ] Disable debug logs by default
- [ ] Portable mode
- [ ] Versioned data migrations
