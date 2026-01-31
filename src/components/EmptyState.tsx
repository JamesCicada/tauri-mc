import { PlusIcon } from "lucide-react";

interface EmptyStateProps {
  onAddClick: () => void;
}

export default function EmptyState({ onAddClick }: EmptyStateProps) {
  return (
    <div className="empty-state">
      <div className="empty-state-icon">
        <svg
          viewBox="0 0 24 24"
          className="empty-state-svg"
          aria-hidden
        >
          <path d="M3 3h7v7H3zM14 3h7v7h-7zM14 14h7v7h-7zM3 14h7v7H3z" />
        </svg>
      </div>
      <h2 className="empty-state-title">No instances yet</h2>
      <p className="empty-state-message">
        Create your first instance by installing a Minecraft version or a modpack from Modrinth.
      </p>
      <button
        type="button"
        className="btn btn-primary empty-state-cta"
        onClick={onAddClick}
      >
        <PlusIcon size={20} strokeWidth={2.5} />
        New instance
      </button>
    </div>
  );
}
