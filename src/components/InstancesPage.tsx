import { PlusIcon } from "lucide-react";
import InstanceCard from "./InstanceCard";
import EmptyState from "./EmptyState";
import type { Instance } from "../types/types";

interface InstancesPageProps {
  instances: Instance[];
  onOpenNewInstance: () => void;
  onPlay: (instance: Instance) => void;
  onLogs: (instance: Instance) => void;
  onSettings: (instance: Instance) => void;
  onViewCrashLogs?: (instance: Instance) => void;
}

export default function InstancesPage({
  instances,
  onOpenNewInstance,
  onPlay,
  onLogs,
  onSettings,
  onViewCrashLogs,
}: InstancesPageProps) {
  return (
    <>
      <section className="instances-page">
        <header className="page-header">
          <h1 className="page-title">Instances</h1>
        </header>

        {instances.length === 0 ? (
          <EmptyState onAddClick={onOpenNewInstance} />
        ) : (
          <div className="instances-grid">
            {instances.map((inst) => (
              <InstanceCard
                key={inst.id}
                instance={inst}
                onPlay={onPlay}
                onLogs={onLogs}
                onSettings={onSettings}
                onViewCrashLogs={onViewCrashLogs}
              />
            ))}
          </div>
        )}
      </section>

      <div className="fab-container">
        <button
          type="button"
          className="fab"
          onClick={onOpenNewInstance}
          title="New instance"
          aria-label="Create new instance"
        >
          <PlusIcon size={28} strokeWidth={2.5} />
        </button>
      </div>
    </>
  );
}
