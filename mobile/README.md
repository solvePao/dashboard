# Dashboard Mobile

Landscape-oriented companion SPA that pairs with the web dashboard via QR code
+ 6-character short code, then talks to the backend with a persistent 6-month
device token.

Stack: Vue 3 (`<script setup>`), TypeScript (strict), Tailwind CSS v4,
Reka UI, Lucide icons, Vite.

## Prerequisites

- Node 18+
- Backend worker URL in `.env` (see `.env.example`):

```bash
VITE_API_URL=http://localhost:8080
```

## Scripts

```bash
npm run dev        # local server on :5174
npm run build      # typecheck + production bundle
npm run preview    # preview the production build
npm run typecheck  # vue-tsc only, no emit
```

## Pairing flow

1. App starts, verifies any saved device token via `GET /api/auth/me`.
2. A rejected token (401/403) is cleared; an unreachable backend keeps the
   saved token and the user can retry via **Check for Auth**.
3. Otherwise a 32-hex session is created (`POST /api/auth/device/session`).
4. The backend's canonical `short_code` and `qr_svg` are displayed (QR via
   `<img>`, with a live expiry countdown from `expires_at`).
5. After pairing on the web dashboard, **Check for Auth** exchanges the
   session for the 6-month token, stored in `localStorage`.

See [architecture.md](./architecture.md) for layers, data flow, and diagrams.
