import { logger } from "../lib/logger";
import { getStoredToken } from "../services/sessionStorage";

const configuredApiUrl = import.meta.env.VITE_API_URL as string | undefined;

export const API_BASE_URL = configuredApiUrl || "http://localhost:8787";

if (!configuredApiUrl && import.meta.env.PROD) {
  logger.warn("api", "VITE_API_URL is not set; falling back to http://localhost:8787");
}

/// Routes that must NOT receive an Authorization header: sign-in and
/// public polling endpoints. Every other route requires the
/// server-minted session JWT.
const PUBLIC_PATHS = new Set([
  "/api/auth/google",
  "/api/auth/device/session",
  "/api/auth/device/check",
]);

export function authHeader(token: string): Record<string, string> {
  return { Authorization: `Bearer ${token}` };
}

export class ApiError extends Error {
  status: number;
  data: unknown;

  constructor(message: string, status: number, data?: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.data = data;
  }
}

export async function apiClient<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const url = `${API_BASE_URL}${path.startsWith("/") ? path : `/${path}`}`;
  const headers = new Headers(options.headers || {});

  // Safety net: authenticated routes always carry the stored JWT unless
  // the caller already set an explicit Authorization header (e.g. tests
  // or a device-token flow).
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  if (!headers.has("Authorization") && !PUBLIC_PATHS.has(normalizedPath)) {
    const stored = getStoredToken();
    if (stored) {
      headers.set("Authorization", `Bearer ${stored}`);
    }
  }

  if (!headers.has("Content-Type") && options.body && typeof options.body === "string") {
    headers.set("Content-Type", "application/json");
  }

  try {
    const res = await fetch(url, {
      ...options,
      headers,
    });

    if (res.status === 204) {
      return {} as T;
    }

    const body = (await res.json().catch(() => null)) as (Record<string, unknown> & { error?: string }) | null;

    if (!res.ok) {
      const errorMessage =
        (typeof body?.error === "string" && body.error) ||
        `Request failed with status ${res.status}`;
      throw new ApiError(errorMessage, res.status, body);
    }

    if (body === null) {
      return {} as T;
    }

    return body as T;
  } catch (err) {
    if (err instanceof ApiError) {
      throw err;
    }
    const isAborted =
      Boolean(options.signal?.aborted) ||
      (err instanceof Error && (err.name === "AbortError" || err.message.toLowerCase().includes("aborted")));
    if (isAborted) {
      throw new ApiError("Request aborted", 0);
    }
    const message = err instanceof Error ? err.message : "Network error occurred";
    logger.error("api", `Failed request to ${path}`, message);
    throw new ApiError(message, 0);
  }
}

