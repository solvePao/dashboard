import { apiClient } from "./client";
import type { SessionResponse, CheckAuthResponse, MeResponse } from "../types/auth";

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

export async function fetchMeApi(token: string): Promise<MeResponse> {
  return apiClient<MeResponse>("/api/auth/me", {
    headers: {
      Authorization: `Bearer ${token}`,
    },
  });
}

