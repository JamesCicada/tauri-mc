import {
  PlayIcon,
  SettingsIcon,
  StopCircleIcon,
  TerminalIcon,
  AlertTriangle,
  Clock,
  Eye,
} from "lucide-react";
import type { Instance } from "../types/types";

interface InstanceCardProps {
  instance: Instance;
  onPlay: (instance: Instance) => void;
  onLogs: (instance: Instance) => void;
  onSettings: (instance: Instance) => void;
  onViewCrashLogs?: (instance: Instance) => void;
}

export default function InstanceCard({
  instance,
  onPlay,
  onLogs,
  onSettings,
  onViewCrashLogs,
}: InstanceCardProps) {
  const formatLastPlayed = (timestamp?: string) => {
    if (!timestamp) return "Never played";
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffDays = Math.floor(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays} days ago`;
    return date.toLocaleDateString();
  };

  const formatPlaytime = (minutes?: number) => {
    if (!minutes || minutes === 0) return null;
    if (minutes < 60) return `${minutes}m`;
    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    if (hours < 24) {
      return remainingMinutes > 0
        ? `${hours}h ${remainingMinutes}m`
        : `${hours}h`;
    }
    const days = Math.floor(hours / 24);
    const remainingHours = hours % 24;
    return remainingHours > 0 ? `${days}d ${remainingHours}h` : `${days}d`;
  };

  return (
    <div
      className={`instance-card ${instance.state === "crashed" ? "crashed" : ""}`}
    >
      <div className="instance-header">
        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              marginBottom: 4,
            }}
          >
            <p className="instance-name">{instance.name}</p>
            {instance.last_crash && (
              <div title="Instance has crashed recently">
                <AlertTriangle size={16} color="var(--error-color)" />
              </div>
            )}
          </div>
          <p className="instance-version">
            {instance.version}
            {instance.loader
              ? ` • ${instance.loader}${instance.loader_version ? ` ${instance.loader_version}` : ""}`
              : ""}
          </p>
          <div className="instance-meta">
            <span className="instance-last-played">
              <Clock size={12} />
              {formatLastPlayed(instance.lastPlayed)}
            </span>
            {formatPlaytime(instance.playtime_minutes) && (
              <span className="instance-playtime">
                {formatPlaytime(instance.playtime_minutes)} played
              </span>
            )}
          </div>
        </div>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "flex-end",
            gap: 8,
          }}
        >
          <span className={`badge badge-${instance.state}`}>
            {instance.state}
          </span>
        </div>
      </div>

      {/* Crash Suggestion */}
      {(instance.state === "crashed" || instance.last_crash) &&
        onViewCrashLogs && (
          <div className="crash-suggestion">
            <div className="crash-suggestion-content">
              <AlertTriangle size={16} color="var(--error-color)" />
              <div className="crash-suggestion-text">
                <span className="crash-suggestion-title">Instance crashed</span>
                <span className="crash-suggestion-message">
                  Check crash logs for troubleshooting information
                </span>
              </div>
            </div>
            <button
              className="btn-secondary btn-sm"
              onClick={() => onViewCrashLogs(instance)}
            >
              <Eye size={14} />
              View Logs
            </button>
          </div>
        )}

      <div
        className="instance-actions"
        style={{
          padding: "16px 0 0 0",
          borderTop: "1px solid rgba(255,255,255,0.05)",
          marginTop: 16,
          justifyContent: "flex-start",
        }}
      >
        <button
          type="button"
          className="btn-transparent"
          onClick={() => onPlay(instance)}
          title={instance.state === "running" ? "Stop" : "Play"}
          style={{ marginRight: 4 }}
        >
          {instance.state === "running" ? (
            <StopCircleIcon color="#ff5252" size={20} strokeWidth={2.5} />
          ) : (
            <PlayIcon
              color="#00c853"
              size={20}
              strokeWidth={2.5}
              fill="#00c853"
            />
          )}
        </button>
        <button
          type="button"
          className="btn-transparent"
          onClick={() => onLogs(instance)}
          title="View logs"
        >
          <TerminalIcon color="var(--text-secondary)" size={18} />
        </button>
        <button
          type="button"
          className="btn-transparent"
          onClick={() => onSettings(instance)}
          title="Manage"
        >
          <SettingsIcon color="var(--text-secondary)" size={18} />
        </button>
      </div>
    </div>
  );
}
