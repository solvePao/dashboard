export interface GoogleCredentialResponse {
  credential: string;
  select_by: string;
}

export interface GoogleIdConfiguration {
  client_id: string;
  callback: (response: GoogleCredentialResponse) => void;
  auto_select?: boolean;
  cancel_on_tap_outside?: boolean;
}

export interface GoogleButtonConfig {
  theme?: "outline" | "filled_blue" | "filled_black";
  size?: "large" | "medium" | "small";
  text?: "signin_with" | "signup_with" | "continue_with" | "signin";
  shape?: "rectangular" | "pill" | "circle" | "square";
  width?: number;
}

export interface GoogleAccountsId {
  initialize: (config: GoogleIdConfiguration) => void;
  renderButton: (element: HTMLElement, config: GoogleButtonConfig) => void;
  disableAutoSelect: () => void;
  revoke: (hint: string, callback: () => void) => void;
}

declare global {
  interface Window {
    google?: {
      accounts: {
        id: GoogleAccountsId;
      };
    };
  }
}
