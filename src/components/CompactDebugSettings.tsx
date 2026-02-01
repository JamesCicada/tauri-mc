import { useState, useEffect } from "react";
import {
  Copy,
  Terminal,
  AlertTriangle,
  Info,
  ChevronDown,
  ChevronRight,
  Monitor,
} from "lucide-react";
import type { Instance, CrashLog, SystemInfo } from "../types/types";
import { invoke } from "@tauri-apps/api/core";

interface CompactDebugSettingsProps {
  instance: Instance;
  onOpenCrashLogs: (instanceId: string) => void;
}

export default function CompactDebugSettings({
  instance,
  onOpenCrashLogs,
}: CompactDebugSettingsProps) {
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [crashLogs, setCrashLogs] = useState<CrashLog[]>([]);
  const [loading, setLoading] = useState(true);
  const [expandedSections, setExpandedSections] = useState<
    Record<string, boolean>
  >({
    system: false,
    crashes: true, // Show crashes by default if any exist
  });

  useEffect(() => {
    loadDebugInfo();
  }, [instance]);

  const loadDebugInfo = async () => {
    try {
      setLoading(true);

      // Get system info
      const sysInfo = await invoke<SystemInfo>("get_system_info");
      setSystemInfo(sysInfo);

      // Get crash logs for this instance
      if (instance.state === "crashed" || instance.last_crash) {
        try {
          const logs = await invoke<CrashLog[]>("get_instance_crash_logs", {
            instanceId: instance.id,
          });
          setCrashLogs(logs);
        } catch (e) {
          console.warn("Could not get crash logs:", e);
        }
      }
    } catch (error) {
      console.error("Failed to load debug info:", error);
    } finally {
      setLoading(false);
    }
  };

  const toggleSection = (section: string) => {
    setExpandedSections((prev) => ({
      ...prev,
      [section]: !prev[section],
    }));
  };

  const copyInstanceInfo = async () => {
    if (!systemInfo) return;

    const info = [
      "=== Instance Debug Info ===",
      `Instance: ${instance.name}`,
      `Version: ${instance.version} (${instance.loader || "Vanilla"})`,
      `State: ${instance.state}`,
      `Last Played: ${instance.lastPlayed || "Never"}`,
      "",
      "=== System Info ===",
      `OS: ${systemInfo.os} ${systemInfo.arch}`,
      `Java: ${systemInfo.java_version || "Not detected"}`,
      `Launcher: v${systemInfo.launcher_version}`,
      "",
      "=== Crashes ===",
      crashLogs.length > 0
        ? crashLogs.map((log) => `${log.crash_type}: ${log.summary}`).join("\n")
        : "No crashes recorded",
    ].join("\n");

    try {
      await navigator.clipboard.writeText(info);
      // TODO: Show toast notification
    } catch (error) {
      console.error("Failed to copy debug info:", error);
    }
  };

  if (loading) {
    return (
      <div className="compact-debug-settings">
        <div className="loading-spinner">Loading debug info...</div>
      </div>
    );
  }

  return (
    <div className="compact-debug-settings">
      <div className="compact-debug-header">
        <h4>Debug Information</h4>
        <button
          className="btn-secondary btn-sm"
          onClick={copyInstanceInfo}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
          }}
        >
          <Copy size={14} />
          Copy Info
        </button>
      </div>

      <div className="compact-debug-sections">
        {/* Instance Status */}
        <div className="compact-debug-summary">
          <div className="instance-debug-status">
            <div className="instance-debug-info">
              <span className="instance-debug-name">{instance.name}</span>
              <span className="instance-debug-version">
                {instance.version} • {instance.loader || "Vanilla"}
              </span>
            </div>
            <span className={`badge badge-${instance.state}`}>
              {instance.state}
            </span>
          </div>

          {instance.last_crash && (
            <div className="crash-alert">
              <AlertTriangle size={14} color="var(--error-color)" />
              <span>Last crash: {instance.last_crash}</span>
              <button
                className="btn-secondary btn-sm"
                onClick={() => onOpenCrashLogs(instance.id)}
              >
                <Terminal size={12} />
                View Logs
              </button>
            </div>
          )}
        </div>

        {/* Collapsible System Info */}
        <div className="compact-debug-section">
          <button
            className="compact-debug-section-header"
            onClick={() => toggleSection("system")}
          >
            {expandedSections.system ? (
              <ChevronDown size={16} />
            ) : (
              <ChevronRight size={16} />
            )}
            <Monitor size={16} />
            <span>System Information</span>
          </button>

          {expandedSections.system && (
            <div className="compact-debug-section-content">
              <div className="compact-system-info">
                <div className="compact-info-row">
                  <span>OS:</span>
                  <span>
                    {systemInfo?.os} {systemInfo?.arch}
                  </span>
                </div>
                <div className="compact-info-row">
                  <span>Java:</span>
                  <span>{systemInfo?.java_version || "Not detected"}</span>
                </div>
                <div className="compact-info-row">
                  <span>Launcher:</span>
                  <span>v{systemInfo?.launcher_version}</span>
                </div>
                {systemInfo?.java_path && (
                  <div className="compact-info-row">
                    <span>Java Path:</span>
                    <span className="compact-path">{systemInfo.java_path}</span>
                  </div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Collapsible Crash Info */}
        {crashLogs.length > 0 && (
          <div className="compact-debug-section">
            <button
              className="compact-debug-section-header"
              onClick={() => toggleSection("crashes")}
            >
              {expandedSections.crashes ? (
                <ChevronDown size={16} />
              ) : (
                <ChevronRight size={16} />
              )}
              <AlertTriangle size={16} />
              <span>Recent Crashes ({crashLogs.length})</span>
            </button>

            {expandedSections.crashes && (
              <div className="compact-debug-section-content">
                <div className="compact-crash-list">
                  {crashLogs.slice(0, 3).map((log, index) => (
                    <div key={index} className="compact-crash-item">
                      <div className="compact-crash-info">
                        <span className="compact-crash-type">
                          {log.crash_type}
                        </span>
                        <span className="compact-crash-summary">
                          {log.summary}
                        </span>
                      </div>
                      <span className="compact-crash-time">
                        {new Date(log.timestamp * 1000).toLocaleDateString()}
                      </span>
                    </div>
                  ))}

                  {crashLogs.length > 3 && (
                    <div className="compact-crash-more">
                      <button
                        className="btn-secondary btn-sm"
                        onClick={() => onOpenCrashLogs(instance.id)}
                      >
                        View All {crashLogs.length} Crashes
                      </button>
                    </div>
                  )}
                </div>
              </div>
            )}
          </div>
        )}

        {/* No crashes message */}
        {crashLogs.length === 0 &&
          instance.state !== "crashed" &&
          !instance.last_crash && (
            <div className="compact-debug-section">
              <div className="compact-no-crashes">
                <Info size={16} color="var(--success-color)" />
                <span>No crashes detected - instance is running smoothly</span>
              </div>
            </div>
          )}
      </div>
    </div>
  );
}
