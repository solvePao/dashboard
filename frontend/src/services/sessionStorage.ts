import { logger } from "../lib/logger";
import type { StoredSession } from "../types/auth";

const STORAGE_KEY = "dashboard.auth.session";

export function decodeExpiry(credential: string): number | null {
  try {
    const payload = credential.split(".")[1];
    if (!payload) return null;
    const normalized = payload.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized + "=".repeat((4 - (normalized.length % 4)) % 4);
    const json = JSON.parse(atob(padded)) as { exp?: unknown };
    if (typeof json.exp === "number") return json.exp * 1000;
    return null;
  } catch {
    logger.debug("auth", "could not decode credential expiry; using fallback lifetime");
    return null;
  }
}

export function persistSession(session: StoredSession | null): void {
  try {
    if (session) {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
    } else {
      localStorage.removeItem(STORAGE_KEY);
    }
  } catch {
    logger.warn("auth", "session storage unavailable; session will not persist");
  }
}

export function restoreSession(): StoredSession | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      logger.debug("auth", "no stored session found");
      return null;
    }
    const session = JSON.parse(raw) as Partial<StoredSession>;
    if (
      !session.user ||
      typeof session.expiresAt !== "number" ||
      session.expiresAt < Date.now()
    ) {
      logger.debug("auth", "ignoring invalid or expired stored session");
      persistSession(null);
      return null;
    }
    return session as StoredSession;
  } catch {
    logger.warn("auth", "stored session unreadable; cleared");
    persistSession(null);
    return null;
  }
}

export function clearSession(): void {
  persistSession(null);
}

