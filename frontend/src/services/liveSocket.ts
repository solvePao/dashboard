import { logger } from "../lib/logger";

/** One envelope on the dashboard's single mux socket (channel `hub`). */
export interface LiveEnvelope {
  v?: number;
  channel: string;
  group_id?: string;
  type?: string;
  [key: string]: unknown;
}

type Handler = (envelope: LiveEnvelope) => void;

/** The dashboard's only WebSocket. Features subscribe by channel. */
class LiveSocketClient {
  private ws: WebSocket | null = null;
  private url = "";
  private wanted = false;
  private handlers = new Set<{ channel: string; handler: Handler }>();
  private reconnectTimer?: ReturnType<typeof setTimeout>;
  private heartbeat?: ReturnType<typeof setInterval>;
  private attempts = 0;

  connect(url: string): void {
    if (this.url === url && this.ws && this.ws.readyState <= WebSocket.OPEN) return;
    this.disconnect();
    this.url = url;
    this.wanted = true;
    this.open();
  }

  disconnect(): void {
    this.wanted = false;
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
      this.heartbeat = setInterval(() => {
        if (this.ws?.readyState === WebSocket.OPEN) this.ws.send(JSON.stringify({ channel: "system", type: "heartbeat" }));
      }, 30_000);
    };
    ws.onmessage = (event) => {
      let data: LiveEnvelope;
      try { data = JSON.parse(String(event.data)) as LiveEnvelope; } catch { return; }
      if (!data.channel) return;
      for (const entry of [...this.handlers]) {
        if (entry.channel !== data.channel) continue;
        try { entry.handler(data); } catch (err) { logger.warn("live", "channel handler failed", err); }
      }
    };
    ws.onclose = () => {
      if (this.ws === ws) this.ws = null;
      clearInterval(this.heartbeat);
      if (this.wanted && this.url) {
        const delay = Math.min(1000 * 2 ** this.attempts, 5000);
        this.attempts += 1;
        this.reconnectTimer = setTimeout(() => this.open(), delay);
      }
    };
    ws.onerror = () => ws.close();
  }

  subscribe(channel: string, handler: Handler): () => void {
    const entry = { channel, handler };
    this.handlers.add(entry);
    return () => { this.handlers.delete(entry); };
  }
}

export const liveSocket = new LiveSocketClient();
