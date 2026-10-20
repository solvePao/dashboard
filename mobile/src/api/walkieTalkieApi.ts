import { apiClient, API_BASE_URL } from "./client";
import type { WalkieDeviceCodeResponse, WalkieGroupSummary } from "../types/walkieTalkie";
import { getStoredDeviceToken } from "../services/deviceStorage";

export class WalkieTalkieApi {
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

  activity(groupId: string, signal: AbortSignal) {
    return this.request<{ ticket: string; group_id: string; group_name: string }>("activity", { group_id: groupId }, signal);
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

  connect(signal: AbortSignal) {
    return this.request<{ ticket: string; group_id: string; group_name: string }>("connect", {}, signal);
  }

  media<T>(connectionId: string, operation: string, body: Record<string, unknown> = {}, signal?: AbortSignal) {
    return this.request<T>("media", { connection_id: connectionId, operation, ...body }, signal);
  }

  socket(groupId: string, ticket: string): WebSocket {
    const url = new URL(`/api/walkie-talkie/socket/${encodeURIComponent(groupId)}`, API_BASE_URL);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    url.searchParams.set("ticket", ticket);
    return new WebSocket(url);
  }
}
