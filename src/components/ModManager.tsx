import { useState, useEffect } from "react";
import {
  RefreshCw,
  Download,
  ToggleLeft,
  ToggleRight,
  Trash2,
  AlertCircle,
} from "lucide-react";
import type { ModFileEntry, ModUpdateInfo } from "../types/types";
import { invoke } from "@tauri-apps/api/core";

interface ModManagerProps {
  instanceId: string;
  onRefresh: () => void;
}

export default function ModManager({ instanceId, onRefresh }: ModManagerProps) {
  const [mods, setMods] = useState<ModFileEntry[]>([]);
  const [updateInfo, setUpdateInfo] = useState<ModUpdateInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);

  useEffect(() => {
    loadMods();
  }, [instanceId]);

  const loadMods = async () => {
    try {
      setLoading(true);
      const modList = await invoke<ModFileEntry[]>("list_instance_mods", {
        instanceId,
      });
      setMods(modList);
    } catch (error) {
      console.error("Failed to load mods:", error);
    } finally {
      setLoading(false);
    }
  };

  const checkForUpdates = async () => {
    try {
      setCheckingUpdates(true);
      const updates = await invoke<ModUpdateInfo[]>("check_mod_updates", {
        instanceId,
      });
      setUpdateInfo(updates);
    } catch (error) {
      console.error("Failed to check for updates:", error);
    } finally {
      setCheckingUpdates(false);
    }
  };

  const toggleMod = async (filename: string, enabled: boolean) => {
    try {
      await invoke("toggle_mod", { instanceId, filename, enabled });
      await loadMods();
      onRefresh();
    } catch (error) {
      console.error("Failed to toggle mod:", error);
    }
  };

  const removeMod = async (filename: string) => {
    if (!confirm(`Are you sure you want to remove ${filename}?`)) {
      return;
    }

    try {
      await invoke("remove_mod", { instanceId, filename });
      await loadMods();
      onRefresh();
    } catch (error) {
      console.error("Failed to remove mod:", error);
    }
  };

  const updateMod = async (modInfo: ModUpdateInfo) => {
    try {
      // Remove old version
      await invoke("remove_mod", { instanceId, filename: modInfo.filename });

      // Install new version
      await invoke("install_modrinth_mod", {
        instanceId,
        projectId: modInfo.project_id,
        versionId: null, // Let it pick the latest compatible version
      });

      await loadMods();
      await checkForUpdates();
      onRefresh();
    } catch (error) {
      console.error("Failed to update mod:", error);
    }
  };

  const getModUpdateInfo = (filename: string) => {
    return updateInfo.find((info) => info.filename === filename);
  };

  const formatFileSize = (bytes: number) => {
    if (bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + " " + sizes[i];
  };

  const isModDisabled = (filename: string) => {
    return filename.endsWith(".disabled");
  };

  const getModDisplayName = (filename: string) => {
    return filename.replace(/\.disabled$/, "").replace(/\.jar$/, "");
  };

  if (loading) {
    return <div className="loading-spinner">Loading mods...</div>;
  }

  return (
    <div className="mod-manager">
      <div className="mod-manager-header">
        <div className="mod-stats">
          <span>{mods.length} mods installed</span>
          {updateInfo.length > 0 && (
            <span className="update-count">
              {updateInfo.filter((info) => info.update_available).length}{" "}
              updates available
            </span>
          )}
        </div>

        <div className="mod-actions">
          <button
            className="btn-secondary"
            onClick={checkForUpdates}
            disabled={checkingUpdates}
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
            }}
          >
            <RefreshCw
              size={16}
              className={checkingUpdates ? "spinning" : ""}
            />
            {checkingUpdates ? "Checking..." : "Check Updates"}
          </button>
        </div>
      </div>

      {mods.length === 0 ? (
        <div className="empty-state">
          <p>No mods installed</p>
          <p className="text-secondary">Add mods to get started</p>
        </div>
      ) : (
        <div className="mod-list">
          {mods.map((mod) => {
            const updateInfo = getModUpdateInfo(mod.name);
            const disabled = isModDisabled(mod.name);
            const displayName = getModDisplayName(mod.name);

            return (
              <div
                key={mod.name}
                className={`mod-item ${disabled ? "disabled" : ""}`}
              >
                <div className="mod-info">
                  <div className="mod-name">
                    {displayName}
                    {disabled && (
                      <span className="disabled-badge">Disabled</span>
                    )}
                  </div>

                  <div className="mod-details">
                    <span className="mod-size">
                      {formatFileSize(mod.size_bytes)}
                    </span>
                    {updateInfo && (
                      <span className="mod-version">
                        v{updateInfo.current_version}
                        {updateInfo.update_available && (
                          <span className="update-available">
                            → v{updateInfo.latest_version}
                          </span>
                        )}
                      </span>
                    )}
                  </div>
                </div>

                <div className="mod-actions">
                  {updateInfo?.update_available && (
                    <button
                      className="btn-primary btn-sm"
                      onClick={() => updateMod(updateInfo)}
                      title="Update mod"
                    >
                      <Download size={14} />
                    </button>
                  )}

                  <button
                    className="btn-icon"
                    onClick={() => toggleMod(mod.name, disabled)}
                    title={disabled ? "Enable mod" : "Disable mod"}
                  >
                    {disabled ? (
                      <ToggleLeft size={18} color="var(--text-secondary)" />
                    ) : (
                      <ToggleRight size={18} color="var(--accent)" />
                    )}
                  </button>

                  <button
                    className="btn-icon btn-danger"
                    onClick={() => removeMod(mod.name)}
                    title="Remove mod"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {updateInfo.some((info) => info.update_available) && (
        <div className="update-summary">
          <AlertCircle size={16} />
          <span>
            {updateInfo.filter((info) => info.update_available).length} mod(s)
            have updates available
          </span>
        </div>
      )}
    </div>
  );
}
