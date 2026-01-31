import { useState } from "react";
import type { ModrinthProjectHit, ModrinthVersion } from "../types/types";

interface ModVersionPickerModalProps {
  hit: ModrinthProjectHit;
  versions: ModrinthVersion[];
  onInstall: (versionId: string) => Promise<void>;
  onCancel: () => void;
}

export default function ModVersionPickerModal({
  hit,
  versions,
  onInstall,
  onCancel,
}: ModVersionPickerModalProps) {
  const [selectedId, setSelectedId] = useState<string>(versions[0]?.id ?? "");
  const selected = versions.find((v) => v.id === selectedId) ?? versions[0];

  return (
    <div
      className="dialog-modal"
      onClick={(e) => e.stopPropagation()}
      style={{ maxWidth: 420, width: "90%" }}
      role="dialog"
      aria-label={`Choose version for ${hit.title}`}
    >
      <h2 className="dialog-title">Choose version: {hit.title}</h2>
      <p
        style={{
          fontSize: "0.85rem",
          color: "var(--text-secondary)",
          marginBottom: 12,
        }}
      >
        Compatible with this instance&apos;s Minecraft version and loader
      </p>
      <div
        style={{
          maxHeight: 240,
          overflowY: "auto",
          border: "1px solid var(--border-color)",
          borderRadius: 8,
          marginBottom: 16,
        }}
      >
        {versions.map((v) => (
          <div
            key={v.id}
            onClick={() => setSelectedId(v.id)}
            style={{
              padding: "10px 12px",
              cursor: "pointer",
              borderBottom:
                v.id !== versions[versions.length - 1].id
                  ? "1px solid var(--border-color)"
                  : "none",
              background: selectedId === v.id ? "var(--bg-hover)" : undefined,
            }}
          >
            <div style={{ fontWeight: 600, fontSize: "0.9rem" }}>
              {v.version_number}
            </div>
            <div
              style={{
                fontSize: "0.75rem",
                color: "var(--text-secondary)",
              }}
            >
              {v.game_versions.join(", ")} · {v.loaders.join(", ")}
            </div>
          </div>
        ))}
      </div>
      <div
        className="dialog-actions"
        style={{ justifyContent: "flex-end", gap: 12 }}
      >
        <button type="button" className="btn btn-secondary" onClick={onCancel}>
          Cancel
        </button>
        <button
          type="button"
          className="btn"
          disabled={!selected}
          onClick={() => selected && onInstall(selected.id)}
        >
          Install {selected?.version_number ?? ""}
        </button>
      </div>
    </div>
  );
}
