import { apiClient } from "./client";
import type { SessionUser } from "../types/auth";

export interface ExchangeGoogleAuthResponse {
  user: SessionUser;
}

export async function exchangeGoogleCredential(
  credential: string
): Promise<ExchangeGoogleAuthResponse> {
  return apiClient<ExchangeGoogleAuthResponse>("/api/auth/google", {
    method: "POST",
    body: JSON.stringify({ credential }),
  });
}

