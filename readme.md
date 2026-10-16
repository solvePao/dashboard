# Dashboard Project

This project contains a Rust backend (Cloudflare Workers) with D1 storage,
plus Vue frontends.

## Running the Backend
The backend is a Rust Worker. To run it locally:
```bash
cd backend && npx wrangler dev --local
```
The worker will serve on http://localhost:8787.

## API
- `POST /api/auth/google` (public sign-in) — body
  `{ "credential": "<Google ID token>" }`. Verifies the token with Google,
  creates/updates the user in D1, and returns
  `{ "user": ..., "token": "<session JWT>", "expires_at": <unix millis> }`.
  This is the only route that accepts a Google ID token.
- All other routes require `Authorization: Bearer <session JWT>` (web) or
  `Authorization: Bearer <device token>` (mobile). A raw `google_sub` is
  never accepted as a credential.
- `GET /api/users/:sub` — session JWT (or device token) whose identity
  matches `sub`. Returns `{ "user": ... }`, or 404.
- `POST /api/auth/device/authorize` — rate-limited to one attempt per
  second per client+code (429 beyond that).

## CORS
The API only answers browsers from allowed origins. Set them with the
comma-separated `FRONTEND_ORIGINS` var (see `backend/.dev.vars.example` for
local dev); without it, only the local Vite servers (ports 5173/5174) are
allowed. When deploying, add the frontend origin, e.g.:
```bash
cd backend
wrangler secret put FRONTEND_ORIGINS
```

## Database (Cloudflare D1)
The worker talks to Cloudflare D1 through a binding (no API token at runtime).
Local secrets go in `backend/.dev.vars` (see `.dev.vars.example`); the D1
database is declared in `backend/wrangler.toml`. Apply the schema locally with:
```bash
cd backend && wrangler d1 execute dashboard --local --file=./schema.sql
```

## Deploying the Backend
```bash
cd backend
wrangler d1 create dashboard
wrangler d1 execute dashboard --remote --file=./schema.sql
wrangler secret put GOOGLE_CLIENT_ID
wrangler secret put JWT_SECRET
wrangler deploy
```
Then point the frontend at the worker URL via `VITE_API_URL`.
