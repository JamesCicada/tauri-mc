import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PackageIcon, SearchIcon, DownloadIcon } from "lucide-react";
import type {
  McVersion,
  VersionManifest,
  ModrinthSearchResult,
  ModrinthProjectHit,
  ModrinthVersion,
} from "../types/types";

type Tab = "version" | "modpack";

type FilterType = "release" | "snapshot" | "old_alpha" | "old_beta" | "all";

interface NewInstanceModalProps {
  open: boolean;
  onClose: () => void;
  onCreateFromVersion: (version: McVersion) => Promise<void>;
  onSelectModpack: (project: ModrinthProjectHit, versionId: string) => void;
  addToast: (message: string, type: "success" | "error") => void;
}

export default function NewInstanceModal({
  open,
  onClose,
  onCreateFromVersion,
  onSelectModpack,
  addToast,
}: NewInstanceModalProps) {
  const [tab, setTab] = useState<Tab>("version");
  const [manifest, setManifest] = useState<VersionManifest | null>(null);
  const [filter, setFilter] = useState<FilterType>("release");
  const [modpackQuery, setModpackQuery] = useState("");
  const [modpackResults, setModpackResults] =
    useState<ModrinthSearchResult | null>(null);
  const [modpackLoading, setModpackLoading] = useState(false);
  const [creating, setCreating] = useState(false);

  const filteredVersions = useMemo(() => {
    if (!manifest) return [];
    if (filter === "all") return manifest.versions;
    return manifest.versions.filter((v: McVersion) => v.type === filter);
  }, [manifest, filter]);

  useEffect(() => {
    if (open) {
      invoke<VersionManifest>("get_version_manifest")
        .then(setManifest)
        .catch(console.error);
    }
  }, [open]);

  const searchModpacks = useCallback(async () => {
    if (!modpackQuery.trim()) return;
    setModpackLoading(true);
    try {
      const results = await invoke<ModrinthSearchResult>("search_projects", {
        query: modpackQuery,
        projectType: "modpack",
      });
      setModpackResults(results);
    } catch (e) {
      addToast(String(e), "error");
    } finally {
      setModpackLoading(false);
    }
  }, [modpackQuery, addToast]);

  const handleCreateVersion = useCallback(
    async (version: McVersion) => {
      setCreating(true);
      try {
        await onCreateFromVersion(version);
        onClose();
      } catch (e) {
        addToast(String(e), "error");
      } finally {
        setCreating(false);
      }
    },
    [onCreateFromVersion, onClose, addToast],
  );

  const handleInstallModpack = useCallback(
    async (hit: ModrinthProjectHit) => {
      try {
        const versions = await invoke<ModrinthVersion[]>("get_project_versions", {
          projectId: hit.project_id,
        });
        const latest = versions[0];
        if (latest) {
          onSelectModpack(hit, latest.id);
          onClose();
        } else {
          addToast("No modpack version found", "error");
        }
      } catch (e) {
        addToast(String(e), "error");
      }
    },
    [onSelectModpack, onClose, addToast],
  );

  if (!open) return null;

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div
        className="dialog-modal new-instance-modal"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 640, width: "90%" }}
        role="dialog"
        aria-labelledby="new-instance-title"
      >
        <h2 id="new-instance-title" className="dialog-title">
          New instance
        </h2>

        <div className="tabs-header new-instance-tabs">
          <button
            type="button"
            className={`tab-item ${tab === "version" ? "active" : ""}`}
            onClick={() => setTab("version")}
          >
            <svg viewBox="0 0 24 24" className="tab-icon" aria-hidden>
              <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5" />
            </svg>
            Vanilla Minecraft
          </button>
          <button
            type="button"
            className={`tab-item ${tab === "modpack" ? "active" : ""}`}
            onClick={() => setTab("modpack")}
          >
            <PackageIcon size={18} className="tab-icon" />
            Modpacks
          </button>
        </div>

        <div className="new-instance-content">
          {tab === "version" && (
            <>
              <div className="filter-bar">
                {(
                  [
                    "release",
                    "snapshot",
                    "old_alpha",
                    "old_beta",
                    "all",
                  ] as const
                ).map((t) => (
                  <button
                    key={t}
                    type="button"
                    className={`btn filter-btn ${filter === t ? "active" : ""}`}
                    onClick={() => setFilter(t)}
                  >
                    {t.replace("_", " ")}
                  </button>
                ))}
              </div>
              {!manifest ? (
                <p style={{ color: "var(--text-secondary)" }}>
                  Connecting to Mojang...
                </p>
              ) : (
                <div
                  className="version-list new-instance-version-list"
                  style={{ maxHeight: 320, overflowY: "auto" }}
                >
                  {filteredVersions.map((v) => (
                    <div key={v.id} className="version-row">
                      <span className="version-id">Minecraft {v.id}</span>
                      <span className="version-type">{v.type}</span>
                      <button
                        type="button"
                        className="btn"
                        onClick={() => handleCreateVersion(v)}
                        disabled={creating}
                      >
                        {creating ? "Creating…" : "Create instance"}
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </>
          )}

          {tab === "modpack" && (
            <>
              <div className="search-container" style={{ marginBottom: 16 }}>
                <input
                  type="text"
                  className="search-input"
                  placeholder="Search modpacks (e.g. Better MC, Fabulously Optimized)..."
                  value={modpackQuery}
                  onChange={(e) => setModpackQuery(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && searchModpacks()}
                />
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={searchModpacks}
                  disabled={modpackLoading}
                >
                  {modpackLoading ? (
                    "Searching..."
                  ) : (
                    <>
                      <SearchIcon size={18} /> Search
                    </>
                  )}
                </button>
              </div>
              {modpackResults && (
                <div
                  className="modpack-grid new-instance-modpack-list"
                  style={{ maxHeight: 320, overflowY: "auto" }}
                >
                  {modpackResults.hits.map((hit) => (
                    <div key={hit.project_id} className="modpack-card">
                      <div className="modpack-info">
                        <img
                          src={
                            hit.icon_url ||
                            "https://cdn.modrinth.com/placeholder.svg"
                          }
                          className="modpack-icon"
                          alt=""
                        />
                        <div className="modpack-details">
                          <h3 className="modpack-title">{hit.title}</h3>
                          <p className="modpack-author">by {hit.author}</p>
                        </div>
                      </div>
                      <p className="modpack-description">{hit.description}</p>
                      <button
                        type="button"
                        className="btn btn-primary"
                        style={{
                          marginTop: "auto",
                          width: "100%",
                          gap: 8,
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                        }}
                        onClick={() => handleInstallModpack(hit)}
                      >
                        <DownloadIcon size={16} /> Install modpack
                      </button>
                    </div>
                  ))}
                </div>
              )}
              {!modpackResults && !modpackLoading && (
                <p
                  style={{
                    color: "var(--text-secondary)",
                    fontSize: "0.9rem",
                    textAlign: "center",
                    padding: 24,
                  }}
                >
                  Search Modrinth to find modpacks. Duplicate names will become
                  &quot;Name (2)&quot;, etc.
                </p>
              )}
            </>
          )}
        </div>

        <div className="dialog-actions" style={{ justifyContent: "flex-end", marginTop: 16 }}>
          <button type="button" className="btn btn-secondary" onClick={onClose}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
