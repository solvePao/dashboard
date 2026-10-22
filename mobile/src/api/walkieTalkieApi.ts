import { apiClient } from "./client";
import type { WalkieDeviceCodeResponse, WalkieGroupSummary } from "../types/walkieTalkie";
import { getStoredDeviceToken } from "../services/deviceStorage";

export class WalkieTalkieApi {
  diagnostic(event: string, status: string): void {
    const token = getStoredDeviceToken();
    if (!token) return;
    void apiClient("/api/walkie-talkie/diagnostics", {
      method: "POST", headers: { Authorization: `Bearer ${token}` },
      body: JSON.stringify({ event, status }), keepalive: true,
    }).catch(() => undefined);
  }

  private request<T>(path: string, body: Record<string, unknown>, signal?: AbortSignal): Promise<T> {
    const token = getStoredDeviceToken();
    if (!token) return Promise.reject(new Error("This phone must be authorized first."));
    return apiClient<T>(`/api/walkie-talkie/${path}`, {
      method: "POST", headers: { Authorization: `Bearer ${token}` }, body: JSON.stringify(body), signal,
    });
  }

  deviceCode(method: "GET" | "POST" | "DELETE", signal?: AbortSignal) {
    const token = getStoredDeviceToken();
    if (!token) return Promise.reject(new Error("This phone must be authorized first."));
    return apiClient<WalkieDeviceCodeResponse>("/api/walkie-talkie/device-code", {
      method, headers: { Authorization: `Bearer ${token}` }, signal,
    });
  }

  memberships(signal?: AbortSignal) {
    return this.request<{ groups: WalkieGroupSummary[]; device_id: string }>("memberships", {}, signal);
  }

  activate(groupId: string) {
    return this.request<{ ok: boolean; cleanup_pending: boolean }>("active", { group_id: groupId });
  }

  wake(groupId: string, signal?: AbortSignal) {
    return this.request<{ ok: boolean; alerted: number }>("wake", { group_id: groupId }, signal);
  }

  availability(signal: AbortSignal) {
    return this.request<{ group_name: string }>("availability", {}, signal);
  }

  media<T>(connectionId: string, operation: string, body: Record<string, unknown> = {}, signal?: AbortSignal) {
    return this.request<T>("media", { connection_id: connectionId, operation, ...body }, signal);
  }
}
