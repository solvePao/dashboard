export interface UserProfile {
  id: number;
  email: string;
  name: string | null;
  picture_url: string | null;
  created_at?: string;
  updated_at?: string;
}

export type AuthState =
  | "idle"
  | "loading"
  | "pending"
  | "checking"
  | "authorized"
  | "expired"
  | "error";

export interface SessionResponse {
  session_id: string;
  short_code: string;
  qr_svg: string;
  expires_at: string;
  error?: string;
}

export interface CheckAuthResponse {
  status: "authorized" | "expired" | "pending" | string;
  token?: string;
  user?: UserProfile;
  error?: string;
}

export interface MeResponse {
  user: UserProfile;
  error?: string;
}

