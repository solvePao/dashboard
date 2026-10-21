import { WalkieTalkieApi } from "../api/walkieTalkieApi";
import type { WalkieGroupSummary, WalkieSnapshot } from "../types/walkieTalkie";

interface Connection { socket: WebSocket; heartbeat?: ReturnType<typeof setInterval> }
export interface GroupActivity { groupId: string; name: string; speaker: string; expiresAt: number }

/** Observes membership and speaking activity only. Never opens media or captures a microphone. */
export class WalkieTalkieActivity {
  private readonly api = new WalkieTalkieApi();
  private connections = new Map<string, Connection>();
  private activity = new Map<string, GroupActivity>();
  private groups: WalkieGroupSummary[] = [];
  private cleanupTimer?: ReturnType<typeof setInterval>;
  private fallbackTimer?: ReturnType<typeof setInterval>;
  private controller = new AbortController();
  private generation = 0;
  private syncing = false;

  private readonly changed: (groups: WalkieGroupSummary[], activity: GroupActivity[]) => void;
  private readonly snapshot: (groupId: string, snapshot: WalkieSnapshot) => void;
  private readonly failed: (message: string) => void;
  private readonly membershipUpdated: (groups: WalkieGroupSummary[]) => void;
  constructor(changed: (groups: WalkieGroupSummary[], activity: GroupActivity[]) => void,
    snapshot: (groupId: string, snapshot: WalkieSnapshot) => void,
    failed: (message: string) => void, membershipUpdated: (groups: WalkieGroupSummary[]) => void = () => undefined) {
    this.changed = changed; this.snapshot = snapshot; this.failed = failed; this.membershipUpdated = membershipUpdated;
  }

  start(): void {
    this.stop();
    this.controller = new AbortController();
    void this.sync();
    this.cleanupTimer = setInterval(() => {
      let changed = false;
      for (const [id, activity] of this.activity) {
        if (activity.expiresAt <= Date.now()) {
          this.activity.delete(id);
          changed = true;
        }
      }
      if (changed) this.emit();
    }, 1000);
    this.fallbackTimer = setInterval(() => {
      if (document.visibilityState === "visible") {
        void this.sync();
      }
    }, 60_000);
  }

  private emit(): void { this.changed(this.groups, [...this.activity.values()]); }

  async sync(): Promise<void> {
    if (this.syncing || this.controller.signal.aborted) return;
    this.syncing = true;
    const generation = this.generation;
    const signal = this.controller.signal;
    try {
      const response = await this.api.memberships(signal);
      if (generation !== this.generation || signal.aborted) return;
      this.groups = response.groups.map(group => {
        const previous = this.groups.find(item => item.id === group.id);
        if (this.connections.get(group.id)?.socket.readyState !== WebSocket.OPEN) return group;
        return { ...group, members: group.members.map(member => {
          const presence = previous?.members.find(item => item.device_id === member.device_id);
          return presence ? { ...member, online: presence.online, audio_active: presence.audio_active } : member;
        }) };
      });
      this.membershipUpdated(this.groups);
      const ids = new Set(this.groups.map(group => group.id));
      for (const [id, connection] of this.connections) {
        if (!ids.has(id)) { this.close(connection); this.connections.delete(id); this.activity.delete(id); }
      }
      this.emit();
      for (const group of this.groups) {
        if (generation !== this.generation || signal.aborted) return;
        if (this.connections.has(group.id)) continue;
        try {
          const ticket = await this.api.activity(group.id, signal);
          if (generation !== this.generation || signal.aborted) return;
          const socket = this.api.socket(group.id, ticket.ticket);
          const connection: Connection = { socket };
          this.connections.set(group.id, connection);
          const timeout = setTimeout(() => { if (socket.readyState !== WebSocket.OPEN) socket.close(); }, 15_000);
          socket.onopen = () => {
            clearTimeout(timeout);
            connection.heartbeat = setInterval(() => {
              if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify({ type: "heartbeat" }));
            }, 10_000);
          };
          socket.onmessage = event => {
            if (generation !== this.generation) return;
            let data: WalkieSnapshot;
            try { data = JSON.parse(String(event.data)) as WalkieSnapshot; } catch { return; }
            if (data.type !== "snapshot") return;
            const current = this.groups.find(item => item.id === group.id);
            if (current) { current.members = data.members; current.name = data.group_name || current.name; }
            this.snapshot(group.id, data);
            if (data.speaker) {
              this.activity.set(group.id, { groupId: group.id, name: data.group_name || group.name,
                speaker: data.clients.find(client => client.connection_id === data.speaker?.connection_id)?.name || "Someone",
                expiresAt: data.speaker.expires_at });
            } else this.activity.delete(group.id);
            this.emit();
          };
          socket.onclose = () => {
            clearTimeout(timeout); clearInterval(connection.heartbeat);
            if (this.connections.get(group.id) !== connection) return;
            this.connections.delete(group.id); this.activity.delete(group.id); this.emit();
          };
          socket.onerror = () => socket.close();
        } catch (error) {
          if (!signal.aborted) this.failed(error instanceof Error ? error.message : "Group activity is unavailable.");
        }
      }
    } catch (error) {
      if (!signal.aborted) this.failed(error instanceof Error ? error.message : "Group activity is unavailable.");
    } finally {
      if (generation === this.generation) this.syncing = false;
    }
  }

  private close(connection: Connection): void {
    clearInterval(connection.heartbeat);
    connection.socket.onmessage = null; connection.socket.onclose = null; connection.socket.onerror = null;
    connection.socket.close();
  }

  stop(): void {
    this.generation++; this.syncing = false;
    clearInterval(this.cleanupTimer); clearInterval(this.fallbackTimer); this.controller.abort();
    this.connections.forEach(connection => this.close(connection)); this.connections.clear();
    this.activity.clear(); this.groups = []; this.emit();
  }
}
