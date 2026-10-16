import { logger } from "../lib/logger";
import type { StoredSession } from "../types/auth";

const STORAGE_KEY = "dashboard.auth.session";

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

function isValidSession(session: Partial<StoredSession>): session is StoredSession {
  return (
    !!session.user &&
    typeof session.expiresAt === "number" &&
    typeof session.token === "string" &&
    session.token.length > 0
  );
}

export function restoreSession(): StoredSession | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      logger.debug("auth", "no stored session found");
      return null;
    }
    const session = JSON.parse(raw) as Partial<StoredSession>;
    if (!isValidSession(session) || session.expiresAt < Date.now()) {
      logger.debug("auth", "ignoring invalid or expired stored session");
      persistSession(null);
      return null;
    }
    return session;
  } catch {
    logger.warn("auth", "stored session unreadable; cleared");
    persistSession(null);
    return null;
  }
}

/// Returns the stored server-minted JWT, or null when signed out/expired.
/// The Google credential is never persisted — it lives only in memory
/// for the duration of the sign-in exchange.
export function getStoredToken(): string | null {
  const session = restoreSession();
  return session ? session.token : null;
}

export function clearSession(): void {
  persistSession(null);
}

export function isStoredSessionExpired(now: number = Date.now()): boolean {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return true;
    const session = JSON.parse(raw) as Partial<StoredSession>;
    return !isValidSession(session) || session.expiresAt <= now;
  } catch {
    return true;
  }
}
