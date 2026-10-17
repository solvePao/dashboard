import { apiClient } from "./client";
import type { SessionUser } from "../types/auth";

export interface ExchangeGoogleAuthResponse {
  user: SessionUser;
  /** Server-minted session JWT — use as `Bearer` on all other routes. */
  token: string;
  /** Unix millis when `token` expires. */
  expires_at: number;
}

export async function exchangeGoogleCredential(
  credential: string
): Promise<ExchangeGoogleAuthResponse> {
  return apiClient<ExchangeGoogleAuthResponse>("/api/auth/google", {
    method: "POST",
    // Stores the HttpOnly hub cookie planted by the login response.
    credentials: "include",
    body: JSON.stringify({ credential }),
  });
}

/** Expires the hub session cookie. Best-effort on sign-out. */
export async function logoutApi(): Promise<void> {
  await apiClient("/api/auth/logout", {
    method: "POST",
    credentials: "include",
  });
}
