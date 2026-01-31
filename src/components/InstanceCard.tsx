import { PlayIcon, SettingsIcon, StopCircleIcon, TerminalIcon } from "lucide-react";
import type { Instance } from "../types/types";

interface InstanceCardProps {
  instance: Instance;
  onPlay: (instance: Instance) => void;
  onLogs: (instance: Instance) => void;
  onSettings: (instance: Instance) => void;
}

export default function InstanceCard({
  instance,
  onPlay,
  onLogs,
  onSettings,
}: InstanceCardProps) {
  return (
    <div className="instance-card">
      <div className="instance-header">
        <div style={{ flex: 1, minWidth: 0 }}>
          <p className="instance-name">{instance.name}</p>
          <p className="instance-version">
            {instance.version}
            {instance.loader
              ? ` • ${instance.loader}${instance.loader_version ? ` ${instance.loader_version}` : ""}`
              : ""}
          </p>
        </div>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "flex-end",
            gap: 8,
          }}
        >
          <span className={`badge badge-${instance.state}`}>{instance.state}</span>
        </div>
      </div>

      <div
        className="instance-actions"
        style={{
          padding: "12px 0 0 0",
          borderTop: "1px solid rgba(255,255,255,0.05)",
          marginTop: 12,
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
