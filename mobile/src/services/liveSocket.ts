import { logger } from "../lib/logger";

/** One envelope on the single mux socket. Payload fields are flattened server-side. */
export interface LiveEnvelope {
  v?: number;
  channel: string;
  group_id?: string;
  type?: string;
  [key: string]: unknown;
}

type Handler = (envelope: LiveEnvelope) => void;

interface WalkieSub { groupId: string; observe: boolean }

/**
 * The phone's only WebSocket. Channels (`device`, `pairing`, `walkie`)
 * are demuxed here; features subscribe instead of opening sockets.
 * One heartbeat keeps every walkie subscription alive server-side.
 */
class LiveSocketClient {
  private ws: WebSocket | null = null;
  private url = "";
  private wanted = false;
  private handlers = new Set<{ channel: string; groupId?: string; handler: Handler }>();
  private walkieSubs = new Map<string, WalkieSub>();
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private heartbeat?: ReturnType<typeof setInterval>;
  private attempts = 0;
  private openWaiters: Array<() => void> = [];

  connect(url: string): void {
    if (this.url === url && this.ws && this.ws.readyState <= WebSocket.OPEN) return;
    this.disconnectSocket();
    this.url = url;
    this.wanted = true;
    this.open();
  }

  disconnect(): void {
    this.wanted = false;
    this.url = "";
    this.walkieSubs.clear();
    this.disconnectSocket();
  }

  private disconnectSocket(): void {
    clearTimeout(this.reconnectTimer);
    clearInterval(this.heartbeat);
    this.attempts = 0;
    if (this.ws) {
      this.ws.onclose = null;
      this.ws.onerror = null;
      this.ws.onmessage = null;
      this.ws.close();
      this.ws = null;
    }
  }

  private open(): void {
    if (!this.url || (this.ws && this.ws.readyState <= WebSocket.OPEN)) return;
    const ws = new WebSocket(this.url);
    this.ws = ws;
    ws.onopen = () => {
      this.attempts = 0;
      clearInterval(this.heartbeat);
      this.heartbeat = setInterval(() => this.sendRaw({ channel: "system", type: "heartbeat" }), 10_000);
      // Re-establish walkie subscriptions and pairing replay on the new socket.
      for (const sub of this.walkieSubs.values()) {
        this.sendRaw({ channel: "walkie", type: "subscribe", group_id: sub.groupId, observe: sub.observe });
      }
      if (this.url.includes("/live/pairing/")) this.sendRaw({ channel: "pairing", type: "status" });
      const waiters = this.openWaiters;
      this.openWaiters = [];
      waiters.forEach((resolve) => resolve());
    };
    ws.onmessage = (event) => {
      let data: LiveEnvelope;
      try { data = JSON.parse(String(event.data)) as LiveEnvelope; } catch { return; }
      if (!data.channel) return;
      for (const entry of [...this.handlers]) {
        if (entry.channel !== data.channel) continue;
        if (entry.groupId && entry.groupId !== data.group_id) continue;
        try { entry.handler(data); } catch (err) { logger.warn("live", "channel handler failed", err); }
      }
    };
    ws.onclose = () => {
      if (this.ws === ws) this.ws = null;
      clearInterval(this.heartbeat);
      // Media sessions must restart against a fresh room connection; the old
      // connection_id is gone server-side once this socket drops.
      for (const entry of [...this.handlers]) {
        if (entry.channel !== "walkie") continue;
        try { entry.handler({ channel: "walkie", type: "socket_closed" }); } catch { /* handler errors are isolated */ }
      }
      if (this.wanted && this.url) {
        const delay = Math.min(1000 * 2 ** this.attempts, 5000);
        this.attempts += 1;
        this.reconnectTimer = setTimeout(() => this.open(), delay);
      }
    };
    ws.onerror = () => ws.close();
  }

  whenOpen(): Promise<void> {
    if (this.ws?.readyState === WebSocket.OPEN) return Promise.resolve();
    return new Promise((resolve) => this.openWaiters.push(resolve));
  }

  subscribe(channel: string, handler: Handler, groupId?: string): () => void {
    const entry = { channel, groupId, handler };
    this.handlers.add(entry);
    return () => { this.handlers.delete(entry); };
  }

  /** Subscribe to one walkie group on the shared socket (observe = presence only). */
  subscribeWalkie(groupId: string, observe: boolean, handler: Handler): () => void {
    const key = `${groupId}:${observe ? "observe" : "media"}`;
    this.walkieSubs.set(key, { groupId, observe });
    const unsubscribe = this.subscribe("walkie", handler, groupId);
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.sendRaw({ channel: "walkie", type: "subscribe", group_id: groupId, observe });
    }
    return () => {
      unsubscribe();
      if (this.walkieSubs.delete(key) && this.ws?.readyState === WebSocket.OPEN) {
        this.sendRaw({ channel: "walkie", type: "unsubscribe", group_id: groupId, observe });
      }
    };
  }

  /** Signalling from a media session: always targets the observe=false subscription. */
  sendWalkie(groupId: string, message: Record<string, unknown>): void {
    this.sendRaw({ channel: "walkie", group_id: groupId, observe: false, ...message });
  }

  sendRaw(value: Record<string, unknown>): void {
    if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify(value));
  }

  get isOpen(): boolean { return this.ws?.readyState === WebSocket.OPEN; }
}

export const liveSocket = new LiveSocketClient();
