import { logger } from "../lib/logger";

export const API_BASE_URL =
  import.meta.env.VITE_API_URL || "http://localhost:8787";

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

  if (!headers.has("Content-Type") && options.body && typeof options.body === "string") {
    headers.set("Content-Type", "application/json");
  }

  try {
    const res = await fetch(url, {
      ...options,
      headers,
    });

    const raw = await res.text().catch(() => "");
    let body: (Record<string, unknown> & { error?: string }) | null = null;
    if (raw) {
      try {
        body = JSON.parse(raw) as Record<string, unknown> & { error?: string };
      } catch {
        body = null;
      }
    }

    if (!res.ok) {
      const errorMessage =
        (body && typeof body.error === "string" && body.error) ||
        `Request failed with status ${res.status}`;
      throw new ApiError(errorMessage, res.status, body);
    }

    if (res.status === 204 || !raw) {
      return undefined as T;
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

