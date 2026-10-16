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
    body: JSON.stringify({ credential }),
  });
}
