# Cloudflare deployment

Production runs in the Solvepao Research account (`eb7edc81b33d5f40d1f026bfd37f3a49`).

| App | Worker | URL |
| --- | --- | --- |
| Frontend | dashboard-homehub | https://homehub.solvepao.com |
| Mobile web | dashboard-nest | https://nest.solvepao.com |
| API | backend-production | https://dashboard-api.solvepao.com |

## Tooling and local validation

Use Node 24, PNPM 12.14.0, and a stable Rust toolchain. Each application keeps its own dependency lockfile and workspace.

```sh
pnpm install --frozen-lockfile
pnpm --dir frontend install --frozen-lockfile
pnpm --dir mobile install --frozen-lockfile
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.8.7 --locked
export VITE_API_URL=https://dashboard-api.solvepao.com
export VITE_GOOGLE_CLIENT_ID='<production client ID>'
pnpm build:frontend
pnpm build:mobile
pnpm build:api
pnpm validate:api
pnpm validate:frontend
pnpm validate:mobile
```

Production Wrangler commands upload the prebuilt API. Always build it before a manual deploy. Local `wrangler dev` still builds automatically. Runtime secrets are never embedded into static assets.

## One-time account setup

1. In Google Cloud project `uday-yadav`, create the Web application OAuth client `Dashboard Production`, with JavaScript origin `https://homehub.solvepao.com`. No redirect URI is required for the existing Google Identity Services credential callback. Keep the existing development client intact. The OAuth audience must remain External / In production.
2. Set production Worker secrets using the explicit environment:

   ```sh
   pnpm exec wrangler secret put GOOGLE_CLIENT_ID --config backend/wrangler.toml --env production
   pnpm exec wrangler secret put JWT_SECRET --config backend/wrangler.toml --env production
   ```

   Use the same production client ID in the frontend and API. Generate the JWT secret using a cryptographically secure generator with at least 32 random bytes. Preserve it across releases. Do not commit secrets or reuse the development JWT secret.
3. Create a dedicated Cloudflare CI token restricted to the production account with Workers Scripts Edit, Workers Routes Edit, D1 Edit, Account Settings Read, and Zone Read for `solvepao.com`. Worker custom domains manage the requested DNS records; normal releases do not need general DNS Edit access.
4. Create the GitHub `production` environment. Set its `CLOUDFLARE_API_TOKEN` secret and `CLOUDFLARE_ACCOUNT_ID` variable. Set the non-secret `GOOGLE_CLIENT_ID` repository variable as well, because the build job reads it before entering the production environment. Do not store the OAuth client secret or JWT secret in the workflow.
5. Before the first migration, export D1 into a private local backup:

   ```sh
   pnpm exec wrangler d1 export dashboard --config backend/wrangler.toml --env production --remote --output /private/tmp/dashboard-before-production.sql
   ```

   Preserve the backup in private storage. It may contain user data and must never be committed or attached to public workflow output. The initial migration uses `CREATE ... IF NOT EXISTS` to preserve existing users while adding missing tables. Inspect existing table definitions before applying it; that clause does not repair incompatible schemas.

## Release pipeline

`.github/workflows/cloudflare.yml` builds and validates all three bundles on pull requests and pushes to `main`. It publishes only pushes to `main` or a manually dispatched run on `main`. Production runs are serialized without cancellation.

The deployment job downloads the validated artifacts, applies pending D1 migrations, deploys the API, checks authentication and CORS, and then deploys both static apps. The final checks verify HTTPS and SPA fallback. A failure stops subsequent steps and fails the workflow; publication across three Workers is sequential, so earlier successful deployments can remain live after a later failure. Version identifiers and the source commit are recorded in the run summary.

Custom domains and certificates are managed by the Wrangler route declarations. The apex site, mail records, and other Workers are independent of these three hostnames. Direct `workers.dev` and preview URLs are disabled for production.

For a manual local release after validation:

```sh
pnpm db:migrate:production
pnpm deploy:api
pnpm verify:production --api-only
pnpm deploy:frontend
pnpm deploy:mobile
pnpm verify:production
```

After the first deployment, visually inspect both UIs, sign in with Google, pair a mobile device, save a widget, observe its live update, and revoke the device. Public HTTP checks alone do not prove those authenticated flows. Widget providers remain external dependencies; check their browser requests for production CORS or availability errors.

## Recovery

List deployment versions and roll back the affected Worker to a recorded version:

```sh
pnpm exec wrangler deployments list --config backend/wrangler.toml --env production
pnpm exec wrangler rollback '<version ID>' --config backend/wrangler.toml --env production
pnpm exec wrangler deployments list --config frontend/wrangler.toml
pnpm exec wrangler rollback '<version ID>' --config frontend/wrangler.toml
pnpm exec wrangler deployments list --config mobile/wrangler.toml
pnpm exec wrangler rollback '<version ID>' --config mobile/wrangler.toml
```

Keep application and database changes backward-compatible. Worker rollback does not undo D1 schema changes, JWT rotation, DNS changes, or Durable Object migration history. Do not delete Durable Object classes to roll back. Inspect the impact and use the private backup or D1 recovery tools deliberately if database restoration is needed.

For runtime diagnosis, use `pnpm exec wrangler tail --config backend/wrangler.toml --env production --status error`. Do not log authorization headers, device tokens, Google ID tokens, or JWT signing secrets.
