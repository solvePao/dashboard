const isDev = import.meta.env.DEV;

const REDACTED = "[REDACTED]";

const SENSITIVE_KEY_PARTS = [
  "token",
  "credential",
  "secret",
  "password",
  "authorization",
  "bearer",
  "jwt",
  "session",
  "email",
  "google",
  "sub",
  "picture",
  "avatar",
  "phone",
  "apikey",
  "api_key",
  "userid",
  "user_id",
  "user",
];

const EMAIL_RE = /[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}/g;
const JWT_RE = /eyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_.-]+/g;
const BEARER_RE = /Bearer\s+[A-Za-z0-9._~+/-]+/gi;

function isSensitiveKey(key: string): boolean {
  const lower = key.toLowerCase().replace(/[^a-z0-9]/g, "");
  return SENSITIVE_KEY_PARTS.some((part) => lower.includes(part.replace(/[^a-z0-9]/g, "")));
}

function sanitizeString(value: string): string {
  return value
    .replace(JWT_RE, REDACTED)
    .replace(BEARER_RE, `Bearer ${REDACTED}`)
    .replace(EMAIL_RE, REDACTED);
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  if (typeof value !== "object" || value === null) return false;
  const proto = Object.getPrototypeOf(value);
  return proto === Object.prototype || proto === null;
}

/** Redacts user-sensitive values so they never reach the devtools console. */
export function sanitizeLogData(value: unknown, depth = 0, seen = new WeakSet<object>()): unknown {
  if (typeof value === "string") return sanitizeString(value);
  if (typeof value !== "object" || value === null) return value;
  if (depth > 6) return REDACTED;
  if (seen.has(value)) return REDACTED;
  seen.add(value);

  if (value instanceof Error) {
    const status =
      "status" in value && typeof (value as { status: unknown }).status === "number"
        ? { status: (value as { status: number }).status }
        : {};
    const data =
      "data" in value ? { data: sanitizeLogData((value as { data: unknown }).data, depth + 1, seen) } : {};
    return { name: value.name, message: sanitizeString(value.message), ...status, ...data };
  }

  if (Array.isArray(value)) {
    return value.map((item) => sanitizeLogData(item, depth + 1, seen));
  }

  if (!isPlainObject(value)) return REDACTED;

  const out: Record<string, unknown> = {};
  for (const [key, entry] of Object.entries(value)) {
    out[key] = isSensitiveKey(key) ? REDACTED : sanitizeLogData(entry, depth + 1, seen);
  }
  return out;
}

function prefix(scope: string, message: string): string {
  return `[${scope}] ${sanitizeString(message)}`;
}

function emit(
  method: "log" | "warn" | "error",
  scope: string,
  message: string,
  data: unknown,
): void {
  // Production consoles must never carry user-sensitive info: stay silent.
  if (!isDev) return;
  if (data === undefined) {
    console[method](prefix(scope, message));
  } else {
    console[method](prefix(scope, message), sanitizeLogData(data));
  }
}

export const logger = {
  debug(scope: string, message: string, data?: unknown): void {
    emit("log", scope, message, data);
  },

  info(scope: string, message: string, data?: unknown): void {
    emit("log", scope, message, data);
  },

  warn(scope: string, message: string, data?: unknown): void {
    emit("warn", scope, message, data);
  },

  error(scope: string, message: string, data?: unknown): void {
    emit("error", scope, message, data);
  },
};
