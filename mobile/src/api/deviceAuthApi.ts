import { apiClient, API_BASE_URL } from "./client";
import type { SessionResponse, CheckAuthResponse, MeResponse } from "../types/auth";

/** WebSocket URL for the pairing push channel of one session. */
export function pairingSocketUrl(sessionId: string): string {
  const base = API_BASE_URL.replace(/^http/, "ws");
  return `${base}/api/auth/device/ws/${sessionId}`;
}

/** WebSocket URL for real-time revocation push channel of an authorized device. */
export function deviceSocketUrl(token: string): string {
  const base = API_BASE_URL.replace(/^http/, "ws");
  return `${base}/api/devices/ws/${token}`;
}

export async function createDeviceSessionApi(
  sessionId: string
): Promise<SessionResponse> {
  return apiClient<SessionResponse>("/api/auth/device/session", {
    method: "POST",
    body: JSON.stringify({ session_id: sessionId }),
  });
}

export async function checkDeviceAuthApi(
  sessionId: string
): Promise<CheckAuthResponse> {
  return apiClient<CheckAuthResponse>("/api/auth/device/check", {
    method: "POST",
    body: JSON.stringify({ session_id: sessionId }),
  });
}

/**
 * Deletes this device on the backend via the shared revoke endpoint:
 * the device token is both the bearer credential and the target, which
 * the backend records as a self-disconnect.
 */
export async function disconnectDeviceApi(token: string): Promise<void> {
  await apiClient(`/api/devices/${encodeURIComponent(token)}`, {
    method: "DELETE",
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
}

export async function fetchMeApi(token: string): Promise<MeResponse> {
  return apiClient<MeResponse>("/api/auth/me", {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
}

