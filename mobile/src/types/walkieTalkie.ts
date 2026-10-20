export interface WalkieMember { device_id: string; user_id: number; name: string; first_name: string; picture_url: string | null; device_name: string; online?: boolean; audio_active?: boolean }
export interface WalkiePeer extends WalkieMember { connection_id: string; published: boolean }
export interface WalkieSpeaker { device_id: string; connection_id: string; user_id: number; expires_at: number }
export interface WalkieSnapshot {
  type: "snapshot";
  group_name: string | null;
  members: WalkieMember[];
  clients: WalkiePeer[];
  speaker: WalkieSpeaker | null;
}
export type WalkieStatus = "activation" | "connecting" | "calling" | "ready" | "talking" | "listening" | "busy" | "reconnecting" | "error";
export interface WalkieState {
  status: WalkieStatus;
  message: string;
  groupName: string;
  peers: WalkiePeer[];
  members: WalkieMember[];
  speaker: WalkieSpeaker | null;
  playbackBlocked: boolean;
}
export interface MediaResponse {
  session_id: string;
  sessionDescription?: RTCSessionDescriptionInit;
  requiresImmediateRenegotiation?: boolean;
}

export interface WalkieGroupSummary { id: string; name: string; members: WalkieMember[]; joined: boolean; active: boolean }

export interface WalkieDeviceCode { code: string; expires_at: number }
export interface WalkieDeviceCodeResponse { device_code: WalkieDeviceCode | null; status: "active" | "used" | "expired" | "none" }
