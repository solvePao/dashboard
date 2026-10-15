export interface DeviceItem {
  token: string;
  user_id: number;
  device_name: string;
  created_at: string;
  expires_at: string;
  last_used_at: string | null;
}

export interface DevicesResponse {
  devices: DeviceItem[];
}

export interface AuthorizeDeviceResponse {
  status?: string;
  message?: string;
  error?: string;
}

export interface AuthorizeDeviceResult {
  success: boolean;
  message: string;
}

