import { apiClient } from "./client";

export interface WalkieTalkieMember {
  device_id: string;
  user_id: number;
  name: string;
  first_name: string;
  picture_url: string | null;
  device_name: string;
  online?: boolean;
  audio_active?: boolean;
}

export interface WalkieTalkieGroup {
  id: string;
  name: string;
  owner_id: number;
  members: WalkieTalkieMember[];
  joined: boolean;
  active: boolean;
}

export interface WalkieTalkieDevicePreview {
  device: WalkieTalkieMember;
  group_id: string;
  group_name: string;
  member_count: number;
}

interface MutationResult {
  ok?: boolean;
  cleanup_pending?: boolean;
}

const base = "/api/walkie-talkie";
const groupPath = (id: string) => `${base}/groups/${encodeURIComponent(id)}`;
const deviceQuery = (deviceToken: string) => `device_token=${encodeURIComponent(deviceToken)}`;

export const walkieTalkieApi = {
  list: (deviceToken: string) => apiClient<{ groups: WalkieTalkieGroup[]; user_id: number; device_id: string }>(`${base}/groups?${deviceQuery(deviceToken)}`),
  create: (name: string) => apiClient<MutationResult & { group_id: string }>(`${base}/groups`, {
    method: "POST", body: JSON.stringify({ name }),
  }),
  previewDevice: (id: string, code: string) => apiClient<WalkieTalkieDevicePreview>(`${groupPath(id)}/device-preview`, {
    method: "POST", body: JSON.stringify({ code }),
  }),
  addDevice: (id: string, code: string, deviceId: string) => apiClient<MutationResult & { group_id: string; device_id: string }>(`${groupPath(id)}/devices`, {
    method: "POST", body: JSON.stringify({ code, device_id: deviceId }),
  }),
  activate: (groupId: string | null, deviceToken: string) => apiClient<MutationResult>(`${base}/active`, {
    method: "POST", body: JSON.stringify({ group_id: groupId, device_token: deviceToken }),
  }),
  rename: (id: string, name: string) => apiClient<MutationResult>(groupPath(id), {
    method: "PATCH", body: JSON.stringify({ name }),
  }),
  delete: (id: string) => apiClient<MutationResult>(groupPath(id), { method: "DELETE" }),
  removeMember: (id: string, deviceId: string, deviceToken: string) => apiClient<MutationResult>(`${groupPath(id)}/members/${encodeURIComponent(deviceId)}?${deviceQuery(deviceToken)}`, { method: "DELETE" }),
};
