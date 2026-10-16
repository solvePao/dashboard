const isDev = import.meta.env.DEV;

function prefix(scope: string, message: string): string {
  return `[${scope}] ${message}`;
}

export const logger = {
  debug(scope: string, message: string, data?: unknown): void {
    if (!isDev) return;
    if (data === undefined) {
      console.log(prefix(scope, message));
    } else {
      console.log(prefix(scope, message), data);
    }
  },

  info(scope: string, message: string, data?: unknown): void {
    if (!isDev) return;
    if (data === undefined) {
      console.log(prefix(scope, message));
    } else {
      console.log(prefix(scope, message), data);
    }
  },

  // warn/error always surface, even in production builds, so backend and
  // auth failures stay visible where dev-only debug/info are silenced.
  warn(scope: string, message: string, data?: unknown): void {
    if (data === undefined) {
      console.error(prefix(scope, message));
    } else {
      console.error(prefix(scope, message), data);
    }
  },

  error(scope: string, message: string, data?: unknown): void {
    if (data === undefined) {
      console.error(prefix(scope, message));
    } else {
      console.error(prefix(scope, message), data);
    }
  },
};

