import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import { liveSocket } from "./liveSocket";
import { sameFields, sameList } from "./walkieTalkieSnapshot";
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
      const groups = response.groups.map(group => {
        const previous = this.groups.find(item => item.id === group.id);
        if (!previous) return group;
        const members = group.members.map(member => {
          const current = previous.members.find(item => item.device_id === member.device_id);
          return current ? { ...member, online: current.online, audio_active: current.audio_active } : member;
        });
        const next = { ...group, members: sameList(previous.members, members, sameFields) ? previous.members : members };
        return sameFields(previous, next) ? previous : next;
      });
      if (!sameList(this.groups, groups, Object.is)) this.groups = groups;
      this.changed(this.groups, response.device_id);
      const ids = new Set(this.groups.map(group => group.id));
      for (const [id, stop] of this.connections) if (!ids.has(id)) { stop(); this.connections.delete(id); }
      for (const id of ids) {
        if (this.connections.has(id)) continue;
        this.connections.set(id, liveSocket.subscribeWalkie(id, true, data => {
          if (generation !== this.generation) return;
          if (data.type === "snapshot") {
            const snapshot = data as unknown as WalkieSnapshot;
            const current = this.groups.find(item => item.id === id);
            if (!current) return;
            const name = snapshot.group_name || current.name;
            const members = sameList(current.members, snapshot.members, sameFields) ? current.members : snapshot.members;
            if (name !== current.name || members !== current.members) {
              this.groups = this.groups.map(group => group.id === id ? { ...group, name, members } : group);
              this.changed(this.groups, response.device_id);
            }
            this.snapshot(id, snapshot);
          } else if (data.type === "ended") {
            // Membership/configuration invalidation may remove an observer.
            // Refresh membership, then establish a new observer if still allowed.
            this.connections.get(id)?.(); this.connections.delete(id); void this.sync();
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
