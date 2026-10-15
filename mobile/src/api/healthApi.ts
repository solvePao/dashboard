import { apiClient } from "./client";
import type { HealthResponse } from "../types/health";

export async function fetchHealthApi(): Promise<HealthResponse> {
  return apiClient<HealthResponse>("/health");
}

