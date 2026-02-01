import { useState, useEffect } from "react";
import { Trash2, HardDrive, RefreshCw, AlertTriangle } from "lucide-react";
import type { CleanupInfo } from "../types/types";
import { invoke } from "@tauri-apps/api/core";

interface CleanupSettingsProps {
  onCleanupComplete: () => void;
}

export default function CleanupSettings({
  onCleanupComplete,
}: CleanupSettingsProps) {
  const [cleanupInfo, setCleanupInfo] = useState<CleanupInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [cleaning, setCleaning] = useState(false);

  useEffect(() => {
    loadCleanupInfo();
  }, []);

  const loadCleanupInfo = async () => {
    try {
      setLoading(true);
      const info = await invoke<CleanupInfo>("get_cleanup_info");
      setCleanupInfo(info);
    } catch (error) {
      console.error("Failed to load cleanup info:", error);
    } finally {
      setLoading(false);
    }
  };

  const cleanupUnusedVersions = async () => {
    if (!cleanupInfo || cleanupInfo.unused_versions.length === 0) return;

    const confirmed = confirm(
      `This will permanently delete ${cleanupInfo.unused_versions.length} unused game version(s). This action cannot be undone. Continue?`,
    );

    if (!confirmed) return;

    try {
      setCleaning(true);
      const cleaned = await invoke<string[]>("cleanup_unused_versions");
      console.log("Cleaned versions:", cleaned);
      await loadCleanupInfo();
      onCleanupComplete();
    } catch (error) {
      console.error("Failed to cleanup versions:", error);
    } finally {
      setCleaning(false);
    }
  };

  const clearAssetCache = async () => {
    if (!cleanupInfo || cleanupInfo.cache_size_mb === 0) return;

    const confirmed = confirm(
      "This will clear the asset cache. Assets will be re-downloaded when needed. Continue?",
    );

    if (!confirmed) return;

    try {
      setCleaning(true);
      const clearedMB = await invoke<number>("clear_asset_cache");
      console.log("Cleared cache:", clearedMB, "MB");
      await loadCleanupInfo();
      onCleanupComplete();
    } catch (error) {
      console.error("Failed to clear cache:", error);
    } finally {
      setCleaning(false);
    }
  };

  const formatSize = (mb: number) => {
    if (mb < 1024) {
      return `${mb.toFixed(1)} MB`;
    }
    return `${(mb / 1024).toFixed(1)} GB`;
  };

  if (loading) {
    return (
      <div className="cleanup-settings">
        <div className="loading-spinner">Loading cleanup information...</div>
      </div>
    );
  }

  if (!cleanupInfo) {
    return (
      <div className="cleanup-settings">
        <div className="error-state">
          <AlertTriangle size={48} color="var(--text-secondary)" />
          <p>Failed to load cleanup information</p>
          <button className="btn-secondary" onClick={loadCleanupInfo}>
            <RefreshCw size={16} />
            Retry
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="cleanup-settings">
      <div className="cleanup-header">
        <h3>Storage Cleanup</h3>
        <p className="text-secondary">
          Free up disk space by removing unused files and clearing caches
        </p>
      </div>

      <div className="cleanup-sections">
        <div className="cleanup-section">
          <div className="cleanup-section-header">
            <div className="cleanup-section-info">
              <h4>Unused Game Versions</h4>
              <p className="text-secondary">
                Game versions not used by any instance
              </p>
            </div>
            <div className="cleanup-section-stats">
              <span className="cleanup-count">
                {cleanupInfo.unused_versions.length} versions
              </span>
            </div>
          </div>

          {cleanupInfo.unused_versions.length > 0 && (
            <div className="cleanup-section-content">
              <div className="unused-versions-list">
                {cleanupInfo.unused_versions.slice(0, 5).map((version) => (
                  <span key={version} className="version-tag">
                    {version}
                  </span>
                ))}
                {cleanupInfo.unused_versions.length > 5 && (
                  <span className="version-tag more">
                    +{cleanupInfo.unused_versions.length - 5} more
                  </span>
                )}
              </div>

              <button
                className="btn btn-danger"
                onClick={cleanupUnusedVersions}
                disabled={cleaning}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                }}
              >
                <Trash2 size={16} />
                {cleaning ? "Cleaning..." : "Remove Unused Versions"}
              </button>
            </div>
          )}

          {cleanupInfo.unused_versions.length === 0 && (
            <div className="cleanup-section-empty">
              <p className="text-secondary">No unused versions found</p>
            </div>
          )}
        </div>

        <div className="cleanup-section">
          <div className="cleanup-section-header">
            <div className="cleanup-section-info">
              <h4>Asset Cache</h4>
              <p className="text-secondary">
                Downloaded game assets and resources
              </p>
            </div>
            <div className="cleanup-section-stats">
              <span className="cleanup-size">
                {formatSize(cleanupInfo.cache_size_mb)}
              </span>
            </div>
          </div>

          {cleanupInfo.cache_size_mb > 0 && (
            <div className="cleanup-section-content">
              <p className="text-secondary">
                Assets will be re-downloaded when needed
              </p>

              <button
                className="btn-secondary"
                onClick={clearAssetCache}
                disabled={cleaning}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: 8,
                  marginTop: 10,
                }}
              >
                <HardDrive size={16} />
                {cleaning ? "Clearing..." : "Clear Asset Cache"}
              </button>
            </div>
          )}

          {cleanupInfo.cache_size_mb === 0 && (
            <div className="cleanup-section-empty">
              <p className="text-secondary">Cache is empty</p>
            </div>
          )}
        </div>
      </div>

      <div className="cleanup-summary">
        <div className="cleanup-total">
          <HardDrive size={20} />
          <span>
            Total potential cleanup: {formatSize(cleanupInfo.total_cleanup_mb)}
          </span>
        </div>

        <button
          className="btn-secondary"
          onClick={loadCleanupInfo}
          disabled={loading || cleaning}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <RefreshCw size={16} className={loading ? "spinning" : ""} />
          Refresh
        </button>
      </div>
    </div>
  );
}
