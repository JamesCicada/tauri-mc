import { useState, useEffect } from "react";
import {
  Copy,
  Terminal,
  AlertTriangle,
  Info,
  HardDrive,
  Monitor,
} from "lucide-react";
import type { Instance, CrashLog, SystemInfo } from "../types/types";
import { invoke } from "@tauri-apps/api/core";
import CrashLogViewer from "./CrashLogViewer";

interface DebugSettingsProps {
  instances: Instance[];
  onOpenCrashLogs: (instanceId: string) => void;
}

export default function DebugSettings({
  instances,
  onOpenCrashLogs,
}: DebugSettingsProps) {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [recentCrashes, setRecentCrashes] = useState<
    Array<{ instance: Instance; crashCount: number }>
  >([]);
  const [selectedCrashInstance, setSelectedCrashInstance] = useState<
    string | null
  >(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadDebugInfo();
  }, [instances]);

  const loadDebugInfo = async () => {
    try {
      setLoading(true);

      // Get system info from backend
      const sysInfo = await invoke<SystemInfo>("get_system_info");
      setSystemInfo(sysInfo);

      // Get crash info for instances
      const crashInfo = [];
      for (const instance of instances) {
        if (instance.state === "crashed" || instance.last_crash) {
          try {
            const crashLogs = await invoke<CrashLog[]>(
              "get_instance_crash_logs",
              { instanceId: instance.id },
            );
            if (crashLogs.length > 0) {
              crashInfo.push({ instance, crashCount: crashLogs.length });
            }
          } catch (e) {
            console.warn(`Could not get crash logs for ${instance.name}:`, e);
          }
        }
      }

      setRecentCrashes(crashInfo.sort((a, b) => b.crashCount - a.crashCount));
    } catch (error) {
      console.error("Failed to load debug info:", error);
    } finally {
      setLoading(false);
    }
  };

  const copySystemInfo = async () => {
    if (!systemInfo) return;

    const info = [
      "=== System Information ===",
      `OS: ${systemInfo.os}`,
      `Architecture: ${systemInfo.arch}`,
      `Memory: ${systemInfo.memory}`,
      `Java Path: ${systemInfo.java_path || "Auto-detected"}`,
      "",
      "=== Instances ===",
      ...instances.map(
        (inst) =>
          `${inst.name}: ${inst.version} (${inst.loader || "Vanilla"}) - ${inst.state}`,
      ),
      "",
      "=== Recent Crashes ===",
      ...recentCrashes.map(
        (crash) => `${crash.instance.name}: ${crash.crashCount} crash(es)`,
      ),
    ].join("\n");

    try {
      await navigator.clipboard.writeText(info);
      // TODO: Show toast notification
    } catch (error) {
      console.error("Failed to copy system info:", error);
    }
  };

  const clearAllCrashLogs = async () => {
    if (
      !confirm("This will clear all crash logs for all instances. Continue?")
    ) {
      return;
    }

    try {
      for (const instance of instances) {
        await invoke("clear_instance_logs", { instanceId: instance.id });
      }
      await loadDebugInfo();
      // TODO: Show toast notification
    } catch (error) {
      console.error("Failed to clear crash logs:", error);
    }
  };

  if (loading) {
    return (
      <div className="debug-settings">
        <div className="loading-spinner">Loading debug information...</div>
      </div>
    );
  }

  return (
    <div className="debug-settings">
      <div className="debug-header">
        <h3>Debug Information</h3>
        <p className="text-secondary">
          System information and crash diagnostics for troubleshooting
        </p>
      </div>

      <div className="debug-sections">
        {/* System Information */}
        <div className="debug-section">
          <div className="debug-section-header">
            <div className="debug-section-info">
              <h4>
                <Monitor size={18} />
                System Information
              </h4>
              <p className="text-secondary">
                Hardware and software environment details
              </p>
            </div>
            <button
              className="btn-secondary"
              onClick={copySystemInfo}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
              }}
            >
              <Copy size={16} className="mr-2" />
              Copy Info
            </button>
          </div>

          <div className="debug-section-content">
            <div className="system-info-grid">
              <div className="system-info-item">
                <span className="system-info-label">Operating System:</span>
                <span className="system-info-value">{systemInfo?.os}</span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Architecture:</span>
                <span className="system-info-value">{systemInfo?.arch}</span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Available Memory:</span>
                <span className="system-info-value">
                  {systemInfo?.total_memory
                    ? `${Math.round(systemInfo.total_memory / 1024 / 1024)} MB`
                    : "Unknown"}
                </span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Java Version:</span>
                <span className="system-info-value">
                  {systemInfo?.java_version || "Not detected"}
                </span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Java Path:</span>
                <span className="system-info-value">
                  {systemInfo?.java_path || "Auto-detected"}
                </span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Launcher Version:</span>
                <span className="system-info-value">
                  {systemInfo?.launcher_version}
                </span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Total Instances:</span>
                <span className="system-info-value">{instances.length}</span>
              </div>
              <div className="system-info-item">
                <span className="system-info-label">Running Instances:</span>
                <span className="system-info-value">
                  {instances.filter((i) => i.state === "running").length}
                </span>
              </div>
            </div>
          </div>
        </div>

        {/* Crash Diagnostics */}
        <div className="debug-section">
          <div className="debug-section-header">
            <div className="debug-section-info">
              <h4>
                <AlertTriangle size={18} />
                Crash Diagnostics
              </h4>
              <p className="text-secondary">Recent crashes and error logs</p>
            </div>
            <div className="debug-section-actions">
              <button className="btn-secondary" onClick={clearAllCrashLogs}>
                Clear All Logs
              </button>
            </div>
          </div>

          <div className="debug-section-content">
            {recentCrashes.length === 0 ? (
              <div className="debug-section-empty">
                <Info size={24} color="var(--text-secondary)" />
                <p>No recent crashes detected</p>
                <p className="text-secondary">
                  All instances are running smoothly
                </p>
              </div>
            ) : (
              <div className="crash-instances-list">
                {recentCrashes.map(({ instance, crashCount }) => (
                  <div key={instance.id} className="crash-instance-item">
                    <div className="crash-instance-info">
                      <div className="crash-instance-name">
                        {instance.name}
                        <span className={`badge badge-${instance.state}`}>
                          {instance.state}
                        </span>
                      </div>
                      <div className="crash-instance-details">
                        <span className="crash-count">
                          {crashCount} crash{crashCount !== 1 ? "es" : ""}{" "}
                          recorded
                        </span>
                        {instance.last_crash && (
                          <span className="last-crash">
                            Last: {instance.last_crash}
                          </span>
                        )}
                      </div>
                    </div>
                    <button
                      className="btn-secondary"
                      onClick={() => onOpenCrashLogs(instance.id)}
                    >
                      <Terminal size={16} />
                      View Logs
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Instance Status Overview */}
        <div className="debug-section">
          <div className="debug-section-header">
            <div className="debug-section-info">
              <h4>
                <HardDrive size={18} />
                Instance Status
              </h4>
              <p className="text-secondary">
                Overview of all instances and their current state
              </p>
            </div>
          </div>

          <div className="debug-section-content">
            <div className="instance-status-grid">
              {instances.map((instance) => (
                <div key={instance.id} className="instance-status-item">
                  <div className="instance-status-info">
                    <span className="instance-status-name">
                      {instance.name}
                    </span>
                    <span className="instance-status-version">
                      {instance.version} • {instance.loader || "Vanilla"}
                    </span>
                  </div>
                  <div className="instance-status-badges">
                    <span className={`badge badge-${instance.state}`}>
                      {instance.state}
                    </span>
                    {instance.last_crash && (
                      <span className="crash-indicator" title="Has crash logs">
                        <AlertTriangle size={12} />
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>

      {selectedCrashInstance && (
        <CrashLogViewer
          instanceId={selectedCrashInstance}
          onClose={() => setSelectedCrashInstance(null)}
        />
      )}
    </div>
  );
}
