import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

export default function PlayTab({ versions }: { versions: any[] }) {
    const [launching, setLaunching] = useState(false);

    async function play(versionId: string) {
        setLaunching(true);
        try {
            await invoke("launch_version", { id: versionId });
        } catch (e) {
            console.error(e);
            alert("Failed to launch");
        } finally {
            setLaunching(false);
        }
    }

    return (
        <div>
            <h2>Play</h2>
            <ul>
                {versions.map((v) => (
                    <li key={v.id}>
                        <button
                            onClick={() => play(v.id)}
                            disabled={launching}
                        >
                            Play {v.id}
                        </button>
                    </li>
                ))}
            </ul>
        </div>
    );
}
