import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { liveSocket } from "./liveSocket";
import type { WalkieGroupSummary, WalkieSnapshot } from "../types/walkieTalkie";

/** Presence and invitations for all memberships, without capturing media. */
export class WalkieTalkieActivity {
  private readonly api = new WalkieTalkieApi();
  private connections = new Map<string, () => void>();
  private groups: WalkieGroupSummary[] = [];
  private timer?: ReturnType<typeof setInterval>;
  private controller = new AbortController();
  private generation = 0;
  private syncing = false;
  private readonly changed: (groups: WalkieGroupSummary[], deviceId: string) => void;
  private readonly snapshot: (groupId: string, snapshot: WalkieSnapshot) => void;
  private readonly failed: (message: string) => void;
  constructor(changed: (groups: WalkieGroupSummary[], deviceId: string) => void,
    snapshot: (groupId: string, snapshot: WalkieSnapshot) => void, failed: (message: string) => void) {
    this.changed = changed; this.snapshot = snapshot; this.failed = failed;
  }

  start(): void {
    this.stop(); this.controller = new AbortController(); void this.sync();
    this.timer = setInterval(() => { if (document.visibilityState === "visible") void this.sync(); }, 60_000);
  }

  async sync(): Promise<void> {
    if (this.syncing || this.controller.signal.aborted) return;
    this.syncing = true;
    const generation = this.generation;
    try {
      const response = await this.api.memberships(this.controller.signal);
      if (generation !== this.generation) return;
      this.groups = response.groups;
      this.changed(this.groups, response.device_id);
      const ids = new Set(this.groups.map(group => group.id));
      for (const [id, stop] of this.connections) if (!ids.has(id)) { stop(); this.connections.delete(id); }
      for (const group of this.groups) {
        if (this.connections.has(group.id)) continue;
        this.connections.set(group.id, liveSocket.subscribeWalkie(group.id, true, data => {
          if (generation !== this.generation) return;
          if (data.type === "snapshot") {
            const snapshot = data as unknown as WalkieSnapshot;
            const current = this.groups.find(item => item.id === group.id);
            if (current) { current.members = snapshot.members; current.name = snapshot.group_name || current.name; }
            this.changed([...this.groups], response.device_id);
            this.snapshot(group.id, snapshot);
          } else if (data.type === "ended") {
            // Membership/configuration invalidation may remove an observer.
            // Refresh membership, then establish a new observer if still allowed.
            this.connections.get(group.id)?.(); this.connections.delete(group.id); void this.sync();
          } else if (data.type === "error") this.failed(String(data.message || "Caller is unavailable."));
        }));
      }
    } catch (error) {
      if (generation === this.generation && !this.controller.signal.aborted) this.failed(error instanceof Error ? error.message : "Caller is unavailable.");
    } finally { if (generation === this.generation) this.syncing = false; }
  }

  stop(): void {
    this.generation++; this.syncing = false; clearInterval(this.timer); this.controller.abort();
    this.connections.forEach(stop => stop()); this.connections.clear(); this.groups = [];
  }
}
