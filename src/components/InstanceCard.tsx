import { Instance } from "../types/types";

export default function InstanceCard({ instance }: { instance: Instance }) {
    return (
        <div className="instance-card">
            <div className="instance-header">
                <strong>{instance.name}</strong>
                <span>{instance.version}</span>
            </div>

            <div className="instance-actions">
                <button disabled={instance.state !== "ready"}>
                    Play
                </button>
                <button>⋮</button>
            </div>
        </div>
    );
}