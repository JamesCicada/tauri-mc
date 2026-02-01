import { useState, useEffect } from "react";
import { Copy, AlertTriangle, Clock, X } from "lucide-react";
import type { CrashLog } from "../types/types";
import { invoke } from "@tauri-apps/api/core";

interface CrashLogViewerProps {
  instanceId: string;
  onClose: () => void;
}

export default function CrashLogViewer({
  instanceId,
  onClose,
}: CrashLogViewerProps) {
  const [crashLogs, setCrashLogs] = useState<CrashLog[]>([]);
  const [selectedLog, setSelectedLog] = useState<CrashLog | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadCrashLogs();
  }, [instanceId]);

  const loadCrashLogs = async () => {
    try {
      setLoading(true);
      const logs = await invoke<CrashLog[]>("get_instance_crash_logs", {
        instanceId,
      });
      setCrashLogs(logs);
      if (logs.length > 0) {
        setSelectedLog(logs[0]);
      }
    } catch (error) {
      console.error("Failed to load crash logs:", error);
    } finally {
      setLoading(false);
    }
  };

  const copyToClipboard = async (content: string) => {
    try {
      await navigator.clipboard.writeText(content);
      // TODO: Show toast notification
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString();
  };

  const getCrashTypeColor = (crashType: string) => {
    switch (crashType.toLowerCase()) {
      case "memory":
        return "#ff5722";
      case "mod conflict":
        return "#ff9800";
      case "java version":
        return "#f44336";
      case "loader issue":
        return "#9c27b0";
      default:
        return "#757575";
    }
  };

  if (loading) {
    return (
      <div className="modal-overlay">
        <div className="modal crash-log-modal">
          <div className="modal-header" style={{ marginBottom: "20px" }}>
            <h2>Crash Logs</h2>
            <button className="btn-icon" onClick={onClose}>
              <X size={20} />
            </button>
          </div>
          <div className="modal-body">
            <div className="loading-spinner">Loading crash logs...</div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="modal-overlay">
      <div className="modal crash-log-modal">
        <div className="modal-header">
          <h2>Crash Logs</h2>
          <button className="btn-icon" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <div className="modal-body crash-log-body">
          {crashLogs.length === 0 ? (
            <div className="empty-state">
              <AlertTriangle size={48} color="var(--text-secondary)" />
              <p>No crash logs found</p>
              <p className="text-secondary">
                This instance hasn't crashed recently
              </p>
            </div>
          ) : (
            <div className="crash-log-container">
              <div className="crash-log-sidebar">
                <h3>Recent Crashes</h3>
                <div className="crash-log-list">
                  {crashLogs.map((log, index) => (
                    <div
                      key={index}
                      className={`crash-log-item ${selectedLog === log ? "selected" : ""}`}
                      onClick={() => setSelectedLog(log)}
                    >
                      <div className="crash-log-item-header">
                        <span
                          className="crash-type-badge"
                          style={{
                            backgroundColor: getCrashTypeColor(log.crash_type),
                          }}
                        >
                          {log.crash_type}
                        </span>
                        <div className="crash-timestamp">
                          <Clock size={12} />
                          {formatTimestamp(log.timestamp)}
                        </div>
                      </div>
                      <p className="crash-summary">{log.summary}</p>
                    </div>
                  ))}
                </div>
              </div>

              <div className="crash-log-content">
                {selectedLog && (
                  <>
                    <div className="crash-log-header">
                      <div className="crash-info">
                        <h3>{selectedLog.crash_type} Crash</h3>
                        <p>{selectedLog.summary}</p>
                        <p className="text-secondary">
                          {formatTimestamp(selectedLog.timestamp)}
                        </p>
                      </div>
                      <button
                        className="btn-secondary"
                        onClick={() => copyToClipboard(selectedLog.content)}
                      >
                        <Copy size={16} />
                        Copy Log
                      </button>
                    </div>

                    <div className="crash-log-text">
                      <pre>{selectedLog.content}</pre>
                    </div>
                  </>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
