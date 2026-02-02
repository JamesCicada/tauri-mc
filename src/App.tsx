import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

import type {
  Instance,
  Settings,
  ModrinthSearchResult,
  ModrinthProjectHit,
  ModrinthVersion,
  LoaderCandidate,
  ScreenshotEntry,
  WorldEntry,
  ServerEntry,
} from "./types/types";
import {
  SearchIcon,
  FolderOpenIcon,
  ImageIcon,
  GlobeIcon,
  ServerIcon,
} from "lucide-react";
import InstancesPage from "./components/InstancesPage";
import NewInstanceModal from "./components/NewInstanceModal";
import ModVersionPickerModal from "./components/ModVersionPickerModal";
import CrashLogViewer from "./components/CrashLogViewer";
import ModManager from "./components/ModManager";
import DebugSettings from "./components/DebugSettings";
import CompactDebugSettings from "./components/CompactDebugSettings";

interface JavaCompatibility {
  compatible: boolean;
  actual_version: number | null;
  required_version: number;
  path: string;
}

/* -------------------- Types -------------------- */

type Page = "instances" | "settings";

/* -------------------- Components -------------------- */

const NavIcon = ({ name }: { name: string }) => {
  switch (name) {
    case "instances":
      return (
        <svg viewBox="0 0 24 24" className="nav-icon">
          <path d="M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z" />
        </svg>
      );
    case "settings":
      return (
        <svg viewBox="0 0 24 24" className="nav-icon">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 11-2.83 2.83l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 112.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1z" />
        </svg>
      );
    default:
      return null;
  }
};

/* -------------------- App -------------------- */

function App() {
  const [page, setPage] = useState<Page>("instances");
  const [instances, setInstances] = useState<Instance[]>([]);
  const [newInstanceModalOpen, setNewInstanceModalOpen] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    instanceId: string;
    instanceName: string;
    versionId: string;
  } | null>(null);
  const [deleteVersion, setDeleteVersion] = useState(false);
  const [_isOnlyInstance, setIsOnlyInstance] = useState(false);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [instanceSettingsModal, setInstanceSettingsModal] =
    useState<Instance | null>(null);
  const [logs, setLogs] = useState<Record<string, string[]>>({});
  const [consoleInstance, setConsoleInstance] = useState<Instance | null>(null);
  const [javaMismatchConfirm, setJavaMismatchConfirm] = useState<{
    instanceId: string;
    actual: number | null;
    required: number;
    path: string;
  } | null>(null);
  const [toasts, setToasts] = useState<
    { id: number; message: string; type: "success" | "error" }[]
  >([]);
  const [settingsTab, setSettingsTab] = useState<
    "general" | "mods" | "screenshots" | "worlds" | "servers" | "debug"
  >("general");
  const [screenshots, setScreenshots] = useState<ScreenshotEntry[]>([]);
  const [worlds, setWorlds] = useState<WorldEntry[]>([]);
  const [servers, setServers] = useState<ServerEntry[]>([]);
  const [addModModalOpen, setAddModModalOpen] = useState(false);
  const [crashLogViewer, setCrashLogViewer] = useState<string | null>(null);
  const [installingModpack, setInstallingModpack] = useState<{
    project: ModrinthProjectHit;
    versionId: string;
  } | null>(null);
  const [newModpackName, setNewModpackName] = useState("");
  const [modSearchQuery, setModSearchQuery] = useState("");
  const [modSearchResults, setModSearchResults] =
    useState<ModrinthSearchResult | null>(null);
  const [modSearchLoading, setModSearchLoading] = useState(false);
  const [modVersionPicker, setModVersionPicker] = useState<{
    hit: ModrinthProjectHit;
    versions: ModrinthVersion[];
  } | null>(null);

  const [loaderCandidates, setLoaderCandidates] = useState<{
    instanceId: string;
    candidates: LoaderCandidate[];
  } | null>(null);
  const [loaderSelectionInstance, setLoaderSelectionInstance] =
    useState<Instance | null>(null);
  const [loaderVersionsModal, setLoaderVersionsModal] = useState<{
    instance: Instance;
    loaderType: string;
    versions: string[];
    includeBeta: boolean;
  } | null>(null);

  const addToast = useCallback(
    (message: string, type: "success" | "error" = "success") => {
      const id = Date.now();
      if (type === "error") {
        console.error("Toast error:", message);
      }
      setToasts((prev) => [...prev, { id, message, type }]);
      setTimeout(
        () => setToasts((prev) => prev.filter((t) => t.id !== id)),
        15000,
      );
    },
    [],
  );

  useEffect(() => {
    invoke<Instance[]>("list_instances")
      .then(setInstances)
      .catch(console.error);

    invoke<Settings>("get_settings").then(setSettings).catch(console.error);

    const unlisten = listen<Instance>("instance-state-changed", (event) => {
      setInstances((prev) =>
        prev.map((i) => (i.id === event.payload.id ? event.payload : i)),
      );
    });

    const unlistenLogs = listen<{ instance_id: string; message: string }>(
      "instance-log",
      (event) => {
        setLogs((prev) => ({
          ...prev,
          [event.payload.instance_id]: [
            ...(prev[event.payload.instance_id] || []),
            event.payload.message,
          ].slice(-1000),
        }));
      },
    );

    // Listen for loader detection from backend when installing modpacks
    const unlistenLoader = listen<string>(
      "modpack-loader-detected",
      (event) => {
        addToast(
          `Modpack requires loader: ${event.payload}. You can install it from the instance settings (Manage → Mods)`,
          "error",
        );
        // Refresh instances so loader info shows in UI
        invoke<Instance[]>("list_instances").then(setInstances);
      },
    );

    // Listen for loader install completion (payload contains instance_id, project_id, version_id)
    const unlistenLoaderInstalled = listen<{
      instance_id: string;
      project_id: string;
      version_id: string;
    }>("loader-installed", (event) => {
      addToast(
        `Loader installed (${event.payload.project_id} ${event.payload.version_id}) for instance ${event.payload.instance_id}`,
        "success",
      );
      invoke<Instance[]>("list_instances").then(setInstances);
    });

    return () => {
      unlisten.then((f) => f());
      unlistenLogs.then((f) => f());
      unlistenLoader.then((f) => f());
      unlistenLoaderInstalled.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (!instanceSettingsModal) return;
    if (settingsTab === "screenshots") {
      invoke<ScreenshotEntry[]>("list_instance_screenshots", {
        instanceId: instanceSettingsModal.id,
      })
        .then(setScreenshots)
        .catch((e) => addToast(String(e), "error"));
    } else if (settingsTab === "worlds") {
      invoke<WorldEntry[]>("list_instance_worlds", {
        instanceId: instanceSettingsModal.id,
      })
        .then(setWorlds)
        .catch((e) => addToast(String(e), "error"));
    } else if (settingsTab === "servers") {
      invoke<ServerEntry[]>("list_instance_servers", {
        instanceId: instanceSettingsModal.id,
      })
        .then(setServers)
        .catch((e) => addToast(String(e), "error"));
    }
  }, [instanceSettingsModal, settingsTab]);

  useEffect(() => {
    // When opening Add Mod modal, show popular mods if no search was performed
    if (addModModalOpen && instanceSettingsModal && !modSearchResults) {
      invoke<ModrinthSearchResult>("get_popular_mods", { limit: 20 })
        .then((res) => setModSearchResults(res))
        .catch((e) => addToast(String(e), "error"));
    }
  }, [addModModalOpen, instanceSettingsModal]);

  const launchAction = useCallback(
    (instanceId: string) => {
      invoke("launch_instance", { instanceId })
        .then(() => addToast("Launching Minecraft...", "success"))
        .catch((e) => {
          console.error(e);
          addToast(String(e), "error");
          invoke<Instance[]>("list_instances").then(setInstances);
        });
    },
    [addToast],
  );

  const killAction = useCallback(
    (instanceId: string) => {
      invoke("kill_instance", { instanceId })
        .then(() => addToast("Process terminated", "success"))
        .catch((e) => addToast(String(e), "error"));
    },
    [addToast],
  );

  const playInstance = useCallback(
    async (instance: Instance) => {
      try {
        if (instance.state === "running") {
          killAction(instance.id);
          return;
        }
        const compatibility = await invoke<JavaCompatibility>(
          "check_java_compatibility",
          { instanceId: instance.id },
        );
        if (compatibility.compatible) {
          launchAction(instance.id);
        } else {
          setJavaMismatchConfirm({
            instanceId: instance.id,
            actual: compatibility.actual_version,
            required: compatibility.required_version,
            path: compatibility.path,
          });
        }
      } catch (e) {
        console.error(e);
        launchAction(instance.id);
      }
    },
    [launchAction, killAction],
  );

  const deleteInstance = useCallback(async () => {
    if (!deleteConfirm) return;
    try {
      await invoke("delete_instance", {
        instanceId: deleteConfirm.instanceId,
        deleteVersion,
      });
      setInstances(await invoke<Instance[]>("list_instances"));
      setDeleteConfirm(null);
      setDeleteVersion(false);
      setIsOnlyInstance(false);
    } catch (e) {
      console.error(e);
    }
  }, [deleteConfirm, deleteVersion]);

  const updateSettings = useCallback(
    async (newSettings: Partial<Settings>) => {
      if (!settings) return console.log(settings);
      const updated = { ...settings, ...newSettings };
      setSettings(updated);
      try {
        await invoke("save_settings", { settings: updated });
      } catch (e) {
        console.error(e);
      }
    },
    [settings],
  );

  const searchMods = useCallback(async () => {
    if (!modSearchQuery) return;
    setModSearchLoading(true);
    try {
      const results = await invoke<ModrinthSearchResult>("search_projects", {
        query: modSearchQuery,
        projectType: "mod",
      });
      setModSearchResults(results);
    } catch (e) {
      addToast(String(e), "error");
    } finally {
      setModSearchLoading(false);
    }
  }, [modSearchQuery, addToast]);

  return (
    <div className="app-layout">
      <aside className="sidebar">
        <div className="sidebar-header">MC Launcher</div>
        <nav className="nav-list">
          <button
            type="button"
            className={`nav-item ${page === "instances" ? "active" : ""}`}
            onClick={() => setPage("instances")}
          >
            <NavIcon name="instances" /> Instances
          </button>
          <button
            type="button"
            className={`nav-item ${page === "settings" ? "active" : ""}`}
            onClick={() => setPage("settings")}
          >
            <NavIcon name="settings" /> Settings
          </button>
        </nav>
      </aside>

      <main className="main-content">
        <div className="page-container">
          {page === "instances" && (
            <InstancesPage
              instances={instances}
              onOpenNewInstance={() => setNewInstanceModalOpen(true)}
              onPlay={playInstance}
              onLogs={setConsoleInstance}
              onSettings={setInstanceSettingsModal}
            />
          )}

          {newInstanceModalOpen && (
            <NewInstanceModal
              open={newInstanceModalOpen}
              onClose={() => setNewInstanceModalOpen(false)}
              onCreateFromVersion={async (version) => {
                const instanceId = await invoke<string>("create_instance", {
                  name: `Minecraft ${version.id}`,
                  version: version.id,
                });
                setInstances(await invoke<Instance[]>("list_instances"));
                await invoke("download_version", {
                  instanceId,
                  versionId: version.id,
                });
                setInstances(await invoke<Instance[]>("list_instances"));
              }}
              onSelectModpack={(project, versionId) => {
                setInstallingModpack({ project, versionId });
                setNewModpackName(project.title);
              }}
              addToast={addToast}
            />
          )}

          {page === "settings" && settings && (
            <section>
              <header className="page-header" style={{ marginBottom: 32 }}>
                <h1 className="page-title">Settings</h1>
              </header>

              <div className="tabs-header" style={{ marginBottom: 0 }}>
                <div
                  className={`tab-item ${settingsTab === "general" ? "active" : ""}`}
                  onClick={() => setSettingsTab("general")}
                >
                  General
                </div>
                <div
                  className={`tab-item ${settingsTab === "debug" ? "active" : ""}`}
                  onClick={() => setSettingsTab("debug")}
                >
                  Debug
                </div>
              </div>

              <div className="tab-content" style={{ padding: "24px" }}>
                {settingsTab === "general" && (
                  <div className="settings-section">
                    <div className="settings-group">
                      <h3 style={{ marginBottom: 24, marginTop: 0 }}>
                        Java Configuration
                      </h3>
                      <div
                        className="settings-row-advanced"
                        style={{ marginBottom: 20 }}
                      >
                        <div className="settings-field">
                          <label>Min Memory (MB)</label>
                          <input
                            type="number"
                            value={settings.min_memory}
                            onChange={(e) =>
                              updateSettings({
                                min_memory: parseInt(e.target.value) || 0,
                              })
                            }
                          />
                        </div>
                        <div className="settings-field">
                          <label>Max Memory (MB)</label>
                          <input
                            type="number"
                            value={settings.max_memory}
                            onChange={(e) =>
                              updateSettings({
                                max_memory: parseInt(e.target.value) || 0,
                              })
                            }
                          />
                        </div>
                      </div>
                      <div
                        className="settings-field"
                        style={{ marginBottom: 20 }}
                      >
                        <label>Global Java Path</label>
                        <input
                          type="text"
                          value={settings.global_java_path || ""}
                          onChange={(e) =>
                            updateSettings({
                              global_java_path: e.target.value || undefined,
                            })
                          }
                        />
                      </div>
                      <div className="settings-field">
                        <label
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 12,
                          }}
                        >
                          <input
                            type="checkbox"
                            checked={settings.skip_java_check || false}
                            onChange={(e) =>
                              updateSettings({
                                skip_java_check: e.target.checked,
                              })
                            }
                          />
                          <span>Skip Java compatibility check (globally)</span>
                        </label>
                      </div>
                    </div>
                  </div>
                )}

                {settingsTab === "debug" && (
                  <div style={{ padding: "24px" }}>
                    <DebugSettings
                      instances={instances}
                      onOpenCrashLogs={(instanceId: string) =>
                        setCrashLogViewer(instanceId)
                      }
                    />
                  </div>
                )}
              </div>
            </section>
          )}
        </div>
      </main>

      {instanceSettingsModal && (
        <div
          className="dialog-overlay"
          onClick={() => setInstanceSettingsModal(null)}
        >
          <div
            className="dialog-modal"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: 650, width: "90%", padding: "32px" }}
          >
            <h2 className="dialog-title" style={{ marginBottom: "20px" }}>
              Manage: {instanceSettingsModal.name}
            </h2>

            <div className="tabs-header">
              <div
                className={`tab-item ${settingsTab === "general" ? "active" : ""}`}
                onClick={() => setSettingsTab("general")}
              >
                General
              </div>
              <div
                className={`tab-item ${settingsTab === "mods" ? "active" : ""}`}
                onClick={() => setSettingsTab("mods")}
              >
                Mods
              </div>
              <div
                className={`tab-item ${settingsTab === "screenshots" ? "active" : ""}`}
                onClick={() => setSettingsTab("screenshots")}
              >
                Screenshots
              </div>
              <div
                className={`tab-item ${settingsTab === "worlds" ? "active" : ""}`}
                onClick={() => setSettingsTab("worlds")}
              >
                Worlds
              </div>
              <div
                className={`tab-item ${settingsTab === "servers" ? "active" : ""}`}
                onClick={() => setSettingsTab("servers")}
              >
                Servers
              </div>
              <div
                className={`tab-item ${settingsTab === "debug" ? "active" : ""}`}
                onClick={() => setSettingsTab("debug")}
              >
                Debug
              </div>
            </div>

            <div className="tab-content">
              {settingsTab === "general" && (
                <div className="settings-section">
                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "1fr 1fr",
                      gap: 16,
                    }}
                  >
                    <div className="settings-field">
                      <label>Min Memory (MB)</label>
                      <input
                        type="number"
                        placeholder="Global default"
                        value={instanceSettingsModal.min_memory || ""}
                        onChange={(e) =>
                          setInstanceSettingsModal({
                            ...instanceSettingsModal,
                            min_memory: parseInt(e.target.value) || undefined,
                          })
                        }
                      />
                    </div>
                    <div className="settings-field">
                      <label>Max Memory (MB)</label>
                      <input
                        type="number"
                        placeholder="Global default"
                        value={instanceSettingsModal.max_memory || ""}
                        onChange={(e) =>
                          setInstanceSettingsModal({
                            ...instanceSettingsModal,
                            max_memory: parseInt(e.target.value) || undefined,
                          })
                        }
                      />
                    </div>
                  </div>
                  <div className="settings-field" style={{ marginTop: 16 }}>
                    <label>Java Path Override</label>
                    <input
                      type="text"
                      placeholder="Global default or auto-detected"
                      value={instanceSettingsModal.java_path_override || ""}
                      onChange={(e) =>
                        setInstanceSettingsModal({
                          ...instanceSettingsModal,
                          java_path_override: e.target.value || undefined,
                        })
                      }
                    />
                  </div>
                  <div className="settings-field" style={{ marginTop: 12 }}>
                    <label
                      style={{ display: "flex", alignItems: "center", gap: 12 }}
                    >
                      <input
                        type="checkbox"
                        checked={instanceSettingsModal.java_warning_ignored}
                        onChange={(e) =>
                          setInstanceSettingsModal({
                            ...instanceSettingsModal,
                            java_warning_ignored: e.target.checked,
                          })
                        }
                      />
                      <span>
                        Skip Java compatibility check for this instance
                      </span>
                    </label>
                  </div>

                  {instanceSettingsModal.loader && (
                    <div className="settings-field" style={{ marginTop: 12 }}>
                      <label>Required Loader</label>
                      <div
                        style={{
                          display: "flex",
                          gap: 8,
                          alignItems: "center",
                        }}
                      >
                        <div style={{ fontWeight: 600 }}>
                          {instanceSettingsModal.loader}
                          {instanceSettingsModal.loader_version
                            ? ` ${instanceSettingsModal.loader_version}`
                            : ""}
                        </div>
                        <button
                          className="btn btn-secondary"
                          onClick={async () => {
                            try {
                              const loaderType = instanceSettingsModal.loader;
                              const mcVersion =
                                instanceSettingsModal.mc_version ||
                                instanceSettingsModal.version;
                              if (!loaderType) {
                                addToast(
                                  "No loader specified for this instance",
                                  "error",
                                );
                                return;
                              }

                              // Fetch stable loader versions (default) and show modal
                              const versions = await invoke<string[]>(
                                "get_loader_versions",
                                { loaderType, mcVersion, includeBeta: false },
                              );
                              if (!versions || versions.length === 0) {
                                addToast(
                                  `No loader versions found for this Minecraft version ${mcVersion}`,
                                  "error",
                                );
                                return;
                              }

                              setLoaderVersionsModal({
                                instance: instanceSettingsModal,
                                loaderType,
                                versions,
                                includeBeta: false,
                              });
                            } catch (e) {
                              addToast(String(e), "error");
                            }
                          }}
                        >
                          Install Loader
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              )}
              {settingsTab === "mods" && (
                <div style={{ padding: 16 }}>
                  <ModManager
                    instanceId={instanceSettingsModal.id}
                    onRefresh={async () => {
                      // Refresh instance data
                      const instances =
                        await invoke<Instance[]>("list_instances");
                      setInstances(instances);
                      const updated = instances.find(
                        (i) => i.id === instanceSettingsModal.id,
                      );
                      if (updated) {
                        setInstanceSettingsModal(updated);
                      }
                    }}
                  />
                </div>
              )}
              {settingsTab === "screenshots" && (
                <div style={{ padding: 16 }}>
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      marginBottom: 16,
                    }}
                  >
                    <span
                      style={{
                        fontSize: "0.9rem",
                        color: "var(--text-secondary)",
                      }}
                    >
                      {screenshots.length} screenshot
                      {screenshots.length !== 1 ? "s" : ""}
                    </span>
                    {instanceSettingsModal && (
                      <button
                        className="btn btn-secondary"
                        onClick={async () => {
                          try {
                            const screenshotsDir = await invoke<string>(
                              "get_instance_screenshots_dir",
                              {
                                instanceId: instanceSettingsModal.id,
                              },
                            );
                            await invoke("open_path", { path: screenshotsDir });
                          } catch (e) {
                            addToast(String(e), "error");
                          }
                        }}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "8px 14px",
                        }}
                      >
                        <FolderOpenIcon size={16} /> Open folder
                      </button>
                    )}
                  </div>
                  <div
                    className="mod-search-results"
                    style={{
                      maxHeight: 320,
                      minHeight: 80,
                      display: "grid",
                      gridTemplateColumns:
                        "repeat(auto-fill, minmax(140px, 1fr))",
                      gap: 12,
                      padding: 12,
                    }}
                  >
                    {screenshots.length === 0 ? (
                      <div
                        style={{
                          gridColumn: "1 / -1",
                          padding: 24,
                          textAlign: "center",
                          color: "var(--text-secondary)",
                          fontSize: "0.9rem",
                        }}
                      >
                        No screenshots yet. Take some in-game (F2).
                      </div>
                    ) : (
                      screenshots.map((s) => (
                        <button
                          key={s.name}
                          className="btn-transparent"
                          onClick={() => invoke("open_path", { path: s.path })}
                          style={{
                            flexDirection: "column",
                            alignItems: "stretch",
                            padding: 8,
                            border: "1px solid var(--border-color)",
                            borderRadius: 8,
                            overflow: "hidden",
                          }}
                        >
                          <div
                            style={{
                              aspectRatio: "16/9",
                              background: "var(--bg-primary)",
                              borderRadius: 4,
                              marginBottom: 8,
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                            }}
                          >
                            <ImageIcon
                              size={32}
                              color="var(--text-secondary)"
                            />
                          </div>
                          <span
                            style={{
                              fontSize: "0.75rem",
                              color: "var(--text-secondary)",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                              textAlign: "left",
                            }}
                          >
                            {s.name}
                          </span>
                        </button>
                      ))
                    )}
                  </div>
                </div>
              )}
              {settingsTab === "worlds" && (
                <div style={{ padding: 16 }}>
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      marginBottom: 16,
                    }}
                  >
                    <span
                      style={{
                        fontSize: "0.9rem",
                        color: "var(--text-secondary)",
                      }}
                    >
                      {worlds.length} world{worlds.length !== 1 ? "s" : ""}
                    </span>
                    {instanceSettingsModal && (
                      <button
                        className="btn btn-secondary"
                        onClick={async () => {
                          try {
                            const savesDir = await invoke<string>(
                              "get_instance_saves_dir",
                              {
                                instanceId: instanceSettingsModal.id,
                              },
                            );
                            await invoke("open_path", { path: savesDir });
                          } catch (e) {
                            addToast(String(e), "error");
                          }
                        }}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "8px 14px",
                        }}
                      >
                        <FolderOpenIcon size={16} /> Open saves folder
                      </button>
                    )}
                  </div>
                  <div
                    className="mod-search-results"
                    style={{
                      maxHeight: 280,
                      minHeight: 80,
                    }}
                  >
                    {worlds.length === 0 ? (
                      <div
                        style={{
                          padding: 24,
                          textAlign: "center",
                          color: "var(--text-secondary)",
                          fontSize: "0.9rem",
                        }}
                      >
                        No worlds yet. Create one in-game.
                      </div>
                    ) : (
                      worlds.map((w) => (
                        <div
                          key={w.folder}
                          className="mod-result-item"
                          style={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "center",
                          }}
                        >
                          <div
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: 12,
                            }}
                          >
                            <GlobeIcon
                              size={24}
                              color="var(--text-secondary)"
                            />
                            <div>
                              <div
                                style={{
                                  fontWeight: 600,
                                  fontSize: "0.9rem",
                                }}
                              >
                                {w.name}
                              </div>
                              <div
                                style={{
                                  fontSize: "0.75rem",
                                  color: "var(--text-secondary)",
                                }}
                              >
                                {w.folder}
                              </div>
                            </div>
                          </div>
                          <button
                            className="btn btn-secondary"
                            style={{ padding: "6px 12px", fontSize: "0.8rem" }}
                            onClick={() =>
                              invoke("open_path", { path: w.path })
                            }
                          >
                            Open folder
                          </button>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              )}
              {settingsTab === "servers" && (
                <div style={{ padding: 16 }}>
                  <div
                    style={{
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      marginBottom: 16,
                    }}
                  >
                    <span
                      style={{
                        fontSize: "0.9rem",
                        color: "var(--text-secondary)",
                      }}
                    >
                      Saved servers (in-game)
                    </span>
                    {instanceSettingsModal && (
                      <button
                        className="btn btn-secondary"
                        onClick={async () => {
                          try {
                            const dir = await invoke<string>(
                              "get_instance_minecraft_dir",
                              {
                                instanceId: instanceSettingsModal.id,
                              },
                            );
                            await invoke("open_path", { path: dir });
                          } catch (e) {
                            addToast(String(e), "error");
                          }
                        }}
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: 8,
                          padding: "8px 14px",
                        }}
                      >
                        <FolderOpenIcon size={16} /> Open .minecraft folder
                      </button>
                    )}
                  </div>
                  <div
                    className="mod-search-results"
                    style={{
                      maxHeight: 280,
                      minHeight: 80,
                    }}
                  >
                    {servers.length === 0 ? (
                      <div
                        style={{
                          padding: 24,
                          textAlign: "center",
                          color: "var(--text-secondary)",
                          fontSize: "0.9rem",
                        }}
                      >
                        No servers listed. Add servers in-game, or open the
                        .minecraft folder to edit servers.dat.
                      </div>
                    ) : (
                      servers.map((s, i) => (
                        <div
                          key={i}
                          className="mod-result-item"
                          style={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "center",
                          }}
                        >
                          <div
                            style={{
                              display: "flex",
                              alignItems: "center",
                              gap: 12,
                            }}
                          >
                            <ServerIcon
                              size={24}
                              color="var(--text-secondary)"
                            />
                            <div>
                              <div
                                style={{
                                  fontWeight: 600,
                                  fontSize: "0.9rem",
                                }}
                              >
                                {s.name}
                              </div>
                              <div
                                style={{
                                  fontSize: "0.75rem",
                                  color: "var(--text-secondary)",
                                }}
                              >
                                {s.ip}
                              </div>
                            </div>
                          </div>
                        </div>
                      ))
                    )}
                  </div>
                </div>
              )}

              {settingsTab === "debug" && instanceSettingsModal && (
                <CompactDebugSettings
                  instance={instanceSettingsModal}
                  onOpenCrashLogs={(instanceId: string) =>
                    setCrashLogViewer(instanceId)
                  }
                />
              )}
            </div>

            <div
              className="dialog-actions"
              style={{ marginTop: 24, justifyContent: "space-between" }}
            >
              <button
                className="btn btn-danger"
                onClick={async () => {
                  const isOnly = await invoke<boolean>("check_version_usage", {
                    instanceId: instanceSettingsModal.id,
                    versionId: instanceSettingsModal.version,
                  });
                  setIsOnlyInstance(isOnly);
                  setDeleteVersion(isOnly);
                  setDeleteConfirm({
                    instanceId: instanceSettingsModal.id,
                    instanceName: instanceSettingsModal.name,
                    versionId: instanceSettingsModal.version,
                  });
                  setInstanceSettingsModal(null);
                }}
              >
                Delete Instance
              </button>
              <div style={{ display: "flex", gap: 12 }}>
                <button
                  className="btn btn-secondary"
                  onClick={() => setInstanceSettingsModal(null)}
                >
                  Cancel
                </button>
                <button
                  className="btn"
                  onClick={async () => {
                    const prevInstance = instances.find(
                      (i) => i.id === instanceSettingsModal.id,
                    );
                    const updatedInstance = { ...instanceSettingsModal };
                    if (
                      prevInstance &&
                      prevInstance.java_path_override !==
                      updatedInstance.java_path_override
                    ) {
                      updatedInstance.java_warning_ignored = false;
                    }
                    await invoke("save_instance", {
                      instance: updatedInstance,
                    });
                    setInstances(await invoke<Instance[]>("list_instances"));
                    setInstanceSettingsModal(null);
                    addToast("Settings saved", "success");
                  }}
                >
                  Save
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {addModModalOpen && instanceSettingsModal && (
        <div
          className="dialog-overlay"
          onClick={(e) => {
            if (e.target === e.currentTarget) setAddModModalOpen(false);
          }}
        >
          <div
            className="dialog-modal"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: 520, width: "90%" }}
          >
            <h2 className="dialog-title">Add mod from Modrinth</h2>
            <div className="search-container" style={{ marginBottom: 16 }}>
              <input
                type="text"
                className="search-input"
                placeholder="Search mods..."
                value={modSearchQuery}
                onChange={(e) => setModSearchQuery(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && searchMods()}
                style={{ padding: "8px 12px", fontSize: "0.9rem" }}
              />
              <button
                className="btn btn-primary"
                onClick={searchMods}
                disabled={modSearchLoading}
                style={{ padding: "8px 16px" }}
              >
                {modSearchLoading ? "..." : <SearchIcon size={16} />}
              </button>
            </div>
            <div
              style={{
                fontSize: "0.85rem",
                color: "var(--text-secondary)",
                marginBottom: 8,
              }}
            >
              {modSearchResults
                ? modSearchQuery
                  ? "Search results"
                  : "Popular Mods"
                : "Search to find mods"}
            </div>
            <div
              className="mod-search-results"
              style={{
                maxHeight: 280,
                marginBottom: 16,
              }}
            >
              {modSearchResults?.hits.map((hit: ModrinthProjectHit) => (
                <div key={hit.project_id} className="mod-result-item">
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 12,
                    }}
                  >
                    <img
                      src={
                        hit.icon_url ||
                        "https://cdn.modrinth.com/placeholder.svg"
                      }
                      style={{
                        width: 32,
                        height: 32,
                        borderRadius: 4,
                      }}
                      alt=""
                    />
                    <div>
                      <div
                        style={{
                          fontWeight: 600,
                          fontSize: "0.9rem",
                        }}
                      >
                        {hit.title}
                      </div>
                      <div
                        style={{
                          fontSize: "0.75rem",
                          color: "var(--text-secondary)",
                        }}
                      >
                        {hit.author}
                      </div>
                    </div>
                  </div>
                  <button
                    className="btn btn-secondary"
                    style={{
                      padding: "4px 10px",
                      fontSize: "0.8rem",
                    }}
                    onClick={async () => {
                      try {
                        const versions = await invoke<ModrinthVersion[]>(
                          "get_compatible_mod_versions",
                          {
                            instanceId: instanceSettingsModal.id,
                            projectId: hit.project_id,
                          },
                        );
                        if (versions.length === 0) {
                          addToast(
                            "No compatible version for this instance",
                            "error",
                          );
                          return;
                        }
                        setModVersionPicker({ hit, versions });
                      } catch (e) {
                        addToast(String(e), "error");
                      }
                    }}
                  >
                    Add
                  </button>
                </div>
              )) ?? (
                  <div
                    style={{
                      padding: 24,
                      textAlign: "center",
                      color: "var(--text-secondary)",
                      fontSize: "0.9rem",
                    }}
                  >
                    Search for mods to add
                  </div>
                )}
            </div>
            <div
              className="dialog-actions"
              style={{ justifyContent: "flex-end" }}
            >
              <button
                className="btn btn-secondary"
                onClick={() => {
                  setAddModModalOpen(false);
                  setModSearchResults(null);
                  setModSearchQuery("");
                }}
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {modVersionPicker && instanceSettingsModal && (
        <div
          className="dialog-overlay"
          onClick={() => setModVersionPicker(null)}
        >
          <ModVersionPickerModal
            hit={modVersionPicker.hit}
            versions={modVersionPicker.versions}
            onInstall={async (versionId: string) => {
              try {
                addToast(
                  `Installing ${modVersionPicker.hit.title}...`,
                  "success",
                );
                await invoke("install_modrinth_mod", {
                  instanceId: instanceSettingsModal.id,
                  projectId: modVersionPicker.hit.project_id,
                  versionId,
                });
                addToast(`${modVersionPicker.hit.title} installed!`, "success");
                setModVersionPicker(null);
                setAddModModalOpen(false);
                // Refresh instances to update mod count
                const instances = await invoke<Instance[]>("list_instances");
                setInstances(instances);
              } catch (e) {
                addToast(String(e), "error");
              }
            }}
            onCancel={() => setModVersionPicker(null)}
          />
        </div>
      )}

      {consoleInstance && (
        <div
          className="dialog-overlay"
          onClick={() => setConsoleInstance(null)}
        >
          <div
            className="dialog-modal console-modal"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="console-header">
              <div className="console-instance-info">
                <h2 className="dialog-title" style={{ marginBottom: 0 }}>
                  {consoleInstance.name}
                </h2>
                <span className={`badge badge-${consoleInstance.state}`}>
                  {consoleInstance.state}
                </span>
              </div>
              <button
                className="btn btn-secondary"
                onClick={() =>
                  setLogs((prev) => ({ ...prev, [consoleInstance.id]: [] }))
                }
              >
                Clear Logs
              </button>
            </div>
            <div className="console-output">
              {(logs[consoleInstance.id] || []).map((line, i) => (
                <div
                  key={i}
                  className={`console-line ${line.toLowerCase().includes("error") ? "error" : ""}`}
                >
                  {line}
                </div>
              ))}
            </div>
            <div className="console-footer">
              <button
                className="btn btn-primary"
                disabled={consoleInstance.state !== "ready"}
                onClick={() => playInstance(consoleInstance)}
              >
                Launch
              </button>
              <button className="btn" onClick={() => setConsoleInstance(null)}>
                Close
              </button>
            </div>
          </div>
        </div>
      )}

      {deleteConfirm && (
        <div className="dialog-overlay" onClick={() => setDeleteConfirm(null)}>
          <div className="dialog-modal" onClick={(e) => e.stopPropagation()}>
            <h2 className="dialog-title">Delete Instance</h2>
            <p className="dialog-message">
              Are you sure you want to delete "{deleteConfirm.instanceName}"?
            </p>
            <label className="dialog-checkbox">
              <input
                type="checkbox"
                checked={deleteVersion}
                onChange={(e) => setDeleteVersion(e.target.checked)}
              />
              <span>Delete version files ({deleteConfirm.versionId})</span>
            </label>
            <div className="dialog-actions">
              <button className="btn" onClick={() => setDeleteConfirm(null)}>
                Cancel
              </button>
              <button className="btn btn-danger" onClick={deleteInstance}>
                Delete
              </button>
            </div>
          </div>
        </div>
      )}

      {javaMismatchConfirm && (
        <div
          className="dialog-overlay"
          onClick={() => setJavaMismatchConfirm(null)}
        >
          <div className="dialog-modal" onClick={(e) => e.stopPropagation()}>
            <h2 className="dialog-title">Java Compatibility</h2>
            <p className="dialog-message">
              This instance requires Java {javaMismatchConfirm.required} but
              found {javaMismatchConfirm.actual ?? "unknown"} at{" "}
              <code style={{ wordBreak: "break-all" }}>
                {javaMismatchConfirm.path}
              </code>
              .
            </p>
            <div style={{ display: "flex", gap: 12, marginTop: 8 }}>
              <button
                className="btn"
                onClick={() => setJavaMismatchConfirm(null)}
              >
                Cancel
              </button>
              <button
                className="btn btn-secondary"
                onClick={() => {
                  // Open instance settings so user can update java path
                  const inst = instances.find(
                    (i) => i.id === javaMismatchConfirm.instanceId,
                  );
                  if (inst) setInstanceSettingsModal(inst);
                  setJavaMismatchConfirm(null);
                }}
              >
                Edit Instance Settings
              </button>
              <button
                className="btn btn-primary"
                onClick={async () => {
                  try {
                    const inst = instances.find(
                      (i) => i.id === javaMismatchConfirm.instanceId,
                    );
                    if (inst) {
                      const updated = { ...inst, java_warning_ignored: true };
                      await invoke("save_instance", { instance: updated });
                      setInstances(await invoke<Instance[]>("list_instances"));
                      setJavaMismatchConfirm(null);
                      launchAction(javaMismatchConfirm.instanceId);
                    }
                  } catch (e) {
                    addToast(String(e), "error");
                  }
                }}
              >
                Ignore and Launch
              </button>
            </div>
          </div>
        </div>
      )}

      {installingModpack && (
        <div
          className="dialog-overlay"
          onClick={() => setInstallingModpack(null)}
        >
          <div className="dialog-modal" onClick={(e) => e.stopPropagation()}>
            <h2 className="dialog-title">Install Modpack</h2>
            <p className="dialog-message">
              Choose a name for your new instance:
            </p>
            <div className="settings-field" style={{ marginBottom: 20 }}>
              <input
                type="text"
                value={newModpackName}
                onChange={(e) => setNewModpackName(e.target.value)}
                placeholder="Instance name..."
                autoFocus
              />
            </div>
            <div className="dialog-actions">
              <button
                className="btn btn-secondary"
                onClick={() => setInstallingModpack(null)}
              >
                Cancel
              </button>
              <button
                className="btn btn-primary"
                onClick={async () => {
                  try {
                    addToast("Starting installation...", "success");
                    const name = newModpackName;
                    const versionId = installingModpack.versionId;
                    setInstallingModpack(null);
                    setPage("instances");

                    // 1. Install modpack files (mods)
                    await invoke("install_modpack_version", {
                      name,
                      versionId,
                    });

                    // 2. Get the instance we just created to get its ID and game version
                    const allInstances =
                      await invoke<Instance[]>("list_instances");
                    const newInst = allInstances.find((i) => i.name === name); // Search by name as a proxy

                    if (newInst) {
                      addToast(
                        `Downloading Minecraft ${newInst.version}...`,
                        "success",
                      );
                      await invoke("download_version", {
                        instanceId: newInst.id,
                        versionId: newInst.version,
                      });

                      // Check if loader is already installed (our improved modpack installation handles this)
                      if (newInst.loader && !newInst.loader_version) {
                        // Only prompt for loader if it's not already installed
                        addToast(
                          `Modpack requires loader: ${newInst.loader}. Searching for compatible loaders...`,
                          "success",
                        );
                        try {
                          const candidates = await invoke<LoaderCandidate[]>(
                            "find_loader_candidates",
                            { instanceId: newInst.id, loader: newInst.loader },
                          );
                          if (candidates && candidates.length === 1) {
                            addToast("Downloading loader...", "success");
                            await invoke("download_loader_version", {
                              instanceId: newInst.id,
                              projectId: candidates[0].project_id,
                              versionId: candidates[0].version_id,
                            });
                          } else if (candidates && candidates.length > 1) {
                            setLoaderCandidates({
                              instanceId: newInst.id,
                              candidates,
                            });
                            setLoaderSelectionInstance(newInst);
                          } else {
                            addToast(
                              "No compatible loader versions found on Modrinth",
                              "error",
                            );
                          }
                        } catch (e) {
                          addToast(String(e), "error");
                        }
                      } else if (newInst.loader && newInst.loader_version) {
                        // Loader already installed during modpack installation
                        addToast(
                          `Loader ${newInst.loader} ${newInst.loader_version} already installed`,
                          "success",
                        );
                      }
                    }

                    addToast("Modpack installed successfully!", "success");
                    setInstances(await invoke<Instance[]>("list_instances"));
                  } catch (e) {
                    addToast(String(e), "error");
                  }
                }}
              >
                Install
              </button>
            </div>
          </div>
        </div>
      )}

      {loaderCandidates && loaderSelectionInstance && (
        <div
          className="dialog-overlay"
          onClick={() => {
            setLoaderCandidates(null);
            setLoaderSelectionInstance(null);
          }}
        >
          <div
            className="dialog-modal"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: 700 }}
          >
            <h2 className="dialog-title">Choose Loader Version</h2>
            <p className="dialog-message">
              Select a loader version to download from Modrinth for{" "}
              <strong>{loaderSelectionInstance.name}</strong> (MC{" "}
              {loaderSelectionInstance.version})
            </p>
            <div
              style={{
                display: "grid",
                gap: 8,
                maxHeight: 300,
                overflow: "auto",
              }}
            >
              {loaderCandidates.candidates.map((c) => (
                <div
                  key={c.version_id}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "8px 12px",
                    border: "1px solid rgba(255,255,255,0.03)",
                    borderRadius: 6,
                  }}
                >
                  <div>
                    <div style={{ fontWeight: 600 }}>
                      {c.project_title} — {c.version_number}
                    </div>
                    <div
                      style={{
                        fontSize: "0.85rem",
                        color: "var(--text-secondary)",
                      }}
                    >
                      Supports: {c.game_versions.join(", ")}
                    </div>
                  </div>
                  <div style={{ display: "flex", gap: 8 }}>
                    <button
                      className="btn"
                      onClick={async () => {
                        try {
                          addToast("Downloading loader...", "success");
                          await invoke("download_loader_version", {
                            instanceId: loaderCandidates.instanceId,
                            projectId: c.project_id,
                            versionId: c.version_id,
                          });
                          setLoaderCandidates(null);
                          setLoaderSelectionInstance(null);
                        } catch (e) {
                          addToast(String(e), "error");
                        }
                      }}
                    >
                      Install
                    </button>
                  </div>
                </div>
              ))}
            </div>
            <div
              style={{
                marginTop: 12,
                display: "flex",
                justifyContent: "flex-end",
                gap: 8,
              }}
            >
              <button
                className="btn btn-secondary"
                onClick={() => {
                  setLoaderCandidates(null);
                  setLoaderSelectionInstance(null);
                }}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {loaderVersionsModal && (
        <div
          className="dialog-overlay"
          onClick={() => setLoaderVersionsModal(null)}
        >
          <div
            className="dialog-modal"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: 600 }}
          >
            <h2 className="dialog-title">
              Install {loaderVersionsModal.loaderType} Loader
            </h2>
            <p className="dialog-message">
              Select a loader version for{" "}
              <strong>{loaderVersionsModal.instance.name}</strong> (MC{" "}
              {loaderVersionsModal.instance.version})
            </p>

            <div
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                marginBottom: 8,
              }}
            >
              <label style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <input
                  type="checkbox"
                  checked={loaderVersionsModal.includeBeta}
                  onChange={async (e) => {
                    const include = e.target.checked;
                    setLoaderVersionsModal({
                      ...loaderVersionsModal,
                      includeBeta: include,
                    });
                    try {
                      const versions = await invoke<string[]>(
                        "get_loader_versions",
                        {
                          loaderType: loaderVersionsModal.loaderType,
                          mcVersion: loaderVersionsModal.instance.version,
                          includeBeta: include,
                        },
                      );
                      setLoaderVersionsModal({
                        ...loaderVersionsModal,
                        includeBeta: include,
                        versions,
                      });
                    } catch (err) {
                      addToast(String(err), "error");
                    }
                  }}
                />
                <span style={{ fontSize: "0.9rem" }}>
                  Show beta/pre-release versions
                </span>
              </label>
            </div>

            <div
              style={{
                display: "grid",
                gap: 8,
                maxHeight: 320,
                overflow: "auto",
              }}
            >
              {loaderVersionsModal.versions.map((v) => (
                <div
                  key={v}
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "8px 12px",
                    border: "1px solid rgba(255,255,255,0.03)",
                    borderRadius: 6,
                  }}
                >
                  <div style={{ fontWeight: 600 }}>{v}</div>
                  <div style={{ display: "flex", gap: 8 }}>
                    <button
                      className="btn"
                      onClick={async () => {
                        try {
                          addToast("Installing loader...", "success");
                          await invoke("install_loader", {
                            loaderType: loaderVersionsModal.loaderType,
                            mcVersion: loaderVersionsModal.instance.version,
                            loaderVersion: v,
                          });
                          setLoaderVersionsModal(null);
                        } catch (err) {
                          addToast(String(err), "error");
                        }
                      }}
                    >
                      Install
                    </button>
                  </div>
                </div>
              ))}
            </div>

            <div
              style={{
                marginTop: 12,
                display: "flex",
                justifyContent: "flex-end",
                gap: 8,
              }}
            >
              <button
                className="btn btn-secondary"
                onClick={() => setLoaderVersionsModal(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {crashLogViewer && (
        <CrashLogViewer
          instanceId={crashLogViewer}
          onClose={() => setCrashLogViewer(null)}
        />
      )}

      <div className="toast-container">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast ${toast.type}`}>
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              {toast.type === "success" ? (
                <div
                  style={{ color: "var(--success-color)", fontSize: "1.2rem" }}
                >
                  ●
                </div>
              ) : (
                <div
                  style={{ color: "var(--error-color)", fontSize: "1.2rem" }}
                >
                  ●
                </div>
              )}
              <span>{toast.message}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
