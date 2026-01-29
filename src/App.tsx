import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

import type { McVersion, VersionManifest } from "./types/types";

/* -------------------- Types -------------------- */

type Page = "instances" | "versions" | "settings";

type InstanceState = "not_installed" | "installing" | "ready" | "running";

interface Instance {
  id: string;
  name: string;
  version: string;
  state: InstanceState;
}

/* -------------------- App -------------------- */

function App() {
  const [page, setPage] = useState<Page>("instances");
  const [manifest, setManifest] = useState<VersionManifest | null>(null);

  const [instances, setInstances] = useState<Instance[]>([]);
  const [filter, setFilter] = useState<
    "release" | "snapshot" | "old_alpha" | "old_beta" | "all"
  >("release");

  /* -------------------- Load manifest -------------------- */

  useEffect(() => {
    invoke<VersionManifest>("get_version_manifest")
      .then(setManifest)
      .catch(console.error);
  }, []);

  /* -------------------- Versions filtering -------------------- */
  useEffect(() => {
    invoke<Instance[]>("list_instances")
      .then(setInstances)
      .catch(console.error);
  }, []);
  const versions = useMemo(() => {
    if (!manifest) return [];
    if (filter === "all") return manifest.versions;
    return manifest.versions.filter((v: McVersion) => v.type === filter);
  }, [manifest, filter]);

  /* -------------------- Instance actions -------------------- */

  const createInstance = async (version: McVersion) => {
    // Backend creates instance + assigns ID
    const instanceId = await invoke<string>("create_instance", {
      name: `Minecraft ${version.id}`,
      version: version.id,
    });

    // Immediately reload instances from disk
    setInstances(await invoke<Instance[]>("list_instances"));

    // Start install (backend updates instance.json)
    await invoke("download_version", {
      instanceId,
      versionId: version.id,
    });

    // Reload again after install completes
    setInstances(await invoke<Instance[]>("list_instances"));
  };

  const playInstance = (instance: Instance) => {
    setInstances((prev) =>
      prev.map((i) =>
        i.id === instance.id ? { ...i, state: "running" } : i
      )
    );

    invoke("launch_instance", { instanceId: instance.id })
      .catch(console.error)
      .finally(() => {
        setInstances((prev) =>
          prev.map((i) =>
            i.id === instance.id ? { ...i, state: "ready" } : i
          )
        );
      });
  };

  /* -------------------- Render -------------------- */

  return (
    <main style={{ padding: 16 }}>
      {/* Navigation */}
      <nav style={{ display: "flex", gap: 8, marginBottom: 16 }}>
        <button onClick={() => setPage("instances")}>Instances</button>
        <button onClick={() => setPage("versions")}>Versions</button>
        <button onClick={() => setPage("settings")}>Settings</button>
      </nav>

      {/* -------------------- INSTANCES -------------------- */}
      {page === "instances" && (
        <section>
          <h1>Instances</h1>

          {instances.length === 0 && (
            <p>No instances yet. Create one from Versions.</p>
          )}

          {instances.map((inst) => (
            <div
              key={inst.id}
              style={{
                padding: 12,
                border: "1px solid #444",
                marginBottom: 8,
              }}
            >
              <strong>{inst.name}</strong>
              <div>Version: {inst.version}</div>
              <div>Status: {inst.state}</div>

              <button
                disabled={inst.state !== "ready"}
                onClick={() => playInstance(inst)}
              >
                Play
              </button>
            </div>
          ))}
        </section>
      )}

      {/* -------------------- VERSIONS -------------------- */}
      {page === "versions" && (
        <section>
          <h1>Versions</h1>

          <div style={{ marginBottom: 8 }}>
            {["release", "snapshot", "old_alpha", "old_beta", "all"].map(
              (t) => (
                <button
                  key={t}
                  onClick={() => setFilter(t as any)}
                  style={{ marginRight: 4 }}
                >
                  {t}
                </button>
              )
            )}
          </div>

          {!manifest ? (
            <p>Loading…</p>
          ) : (
            <div>
              {versions.map((v: McVersion) => (
                <button
                  key={v.id}
                  style={{
                    display: "block",
                    marginBottom: 6,
                    padding: 8,
                  }}
                  onClick={() => createInstance(v)}
                >
                  Create Instance – {v.id} ({v.type})
                </button>
              ))}
            </div>
          )}
        </section>
      )}

      {/* -------------------- SETTINGS -------------------- */}
      {page === "settings" && (
        <section>
          <h1>Settings</h1>
          <p>Coming later.</p>
        </section>
      )}
    </main>
  );
}

export default App;
