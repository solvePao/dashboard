export interface SessionUser {
  id: number;
  google_sub: string;
  email: string;
  name: string | null;
  picture_url: string | null;
  created_at: string;
  updated_at: string;
  last_login_at: string | null;
}

export interface StoredSession {
  user: SessionUser;
  expiresAt: number;
  /** Server-minted session JWT sent as `Bearer` on authenticated routes. */
  token: string;
}

