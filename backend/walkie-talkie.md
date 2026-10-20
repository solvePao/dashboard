# Walkie-talkie setup and validation

Groups are owned by accounts and start empty. Every phone is already authorized to its own account and has the walkie-talkie widget enabled through that account's dashboard. The phone generates a single-use device code inside the widget; the user shares it with the group owner, who previews the device and confirms admission through Group settings beneath the group name.

A group has at most five joined devices. Device memberships are independent and persist while offline or paused. A phone may join multiple groups but uses one active group. Admission preserves an existing active selection; the first membership becomes active. Microphone capture and playback still require local activation. Mobile shows profile first names and photos with avatar fallbacks; dashboard management also shows device names.

## Configure Cloudflare

1. Create two Cloudflare Realtime SFU applications: development and production. Do not reuse one application's credentials across environments.
2. Supply `WALKIE_SFU_APP_ID` and `WALKIE_SFU_APP_SECRET` on the backend. Local development can use ignored `backend/.dev.vars`. For production, put the app ID in `[env.production.vars]` of `backend/wrangler.toml` and install the secret with `wrangler secret put WALKIE_SFU_APP_SECRET --env production`, from `backend/`.
3. For networks requiring a TURN relay, create a Cloudflare TURN key. Configure `WALKIE_TURN_KEY_ID` and secret `WALKIE_TURN_API_TOKEN` on the backend. Without that configuration the browser uses Cloudflare STUN; restricted networks may not connect. Production should configure TURN. The browser receives only generated, one-hour credentials. Each reconnect fetches fresh credentials.
4. Apply D1 migrations locally with `wrangler d1 migrations apply dashboard --local`, from `backend/`. Apply production migrations through the repository's existing production migration command. Migration 0003 introduced device membership. Migration 0004 intentionally clears every walkie membership, active selection, old group invite, and widget group identifier, while preserving groups, ownership, application authorization, and widget enablement. Back up the database before applying this agreed reset; `make start-api` and `make db-schema-local` now apply migrations as well as the baseline schema, so an existing local database receives new feature tables automatically.
5. Build the Worker from `backend/` with `worker-build --release --locked`. The new `WALKIE_GROUP` Durable Object binding is declared in both environments, with migration `v4`.
6. Deploy the backend before the frontend and mobile builds. Frontend and mobile keep their existing `VITE_API_URL` settings and origins. Microphone access on phones requires HTTPS.

No audio storage, recording service, transcription service, or external media server is used. Cloudflare terminates standard WebRTC transport encryption at the SFU; this is not additional end-to-end group encryption.

## Behavior and interfaces

- `GET /api/walkie-talkie/groups?device_token=…`: dashboard-scoped memberships plus account-owned groups, public device ID, joined/active flags, names/photos, and device presence. Authentication credentials are never returned in rosters or previews.
- `POST /api/walkie-talkie/groups`: `{name}` creates an empty group owned by the account. No phone is enrolled or selected automatically.
- `PATCH/DELETE /api/walkie-talkie/groups/:group`: owner rename/delete. Deletion unregisters every group device, clears affected active selections, closes participation, and queues SFU cleanup. Device authorization and other memberships remain intact.
- `GET/POST/DELETE /api/walkie-talkie/device-code`: device authentication only; retrieve, generate/replace, or cancel this enabled phone's code. Responses contain `{device_code:{code,expires_at}|null,status}` with status `active`, `used`, `expired`, or `none`. Codes are six digits, single use, valid for ten minutes, collision-checked and rate-limited.
- `POST /api/walkie-talkie/groups/:group/device-preview`: owner-only `{code}` returns `{device,group_id,group_name,member_count}`. The public device preview contains first name, photo, device name, and public device ID. Preview never reserves or consumes the code.
- `POST /api/walkie-talkie/groups/:group/devices`: owner-only `{code,device_id}` confirms the previewed device. A D1 transaction revalidates owner, code, exact device, authorization, enabled widget, and five-device capacity; admits membership, establishes only the first active selection, and consumes the code. Full groups roll back without consuming codes or changing selection. Receipt retries require the same surviving membership in the same group; consumed codes cannot recreate removed membership or admit another group.
- Legacy `POST /join` and `POST/DELETE /groups/:group/invite` return 410, explaining the phone-generated code flow.
- `DELETE /api/walkie-talkie/groups/:group/members/:device_id`: owner removes a phone; an account may make its own device leave. Ownership survives the owner's phones leaving.
- `POST /api/walkie-talkie/active`: `{group_id,device_token}` for dashboard sessions, `{group_id}` for authenticated phones. Only joined groups can be activated. Switching mutes previous audio and requires a fresh talk press. Clearing selection preserves membership.
- Existing full-set widget PUT stores `{widget_key:"walkie_talkie",enabled:true,config:{group_id}}`. Selection requires membership. Disablement ends live participation without removing memberships.
- `POST /api/walkie-talkie/memberships`: this phone's memberships and active selection. Foreground polling detects revoked membership and refreshes widget configuration, including after the reset migration.
- `POST /api/walkie-talkie/activity`: `{group_id}` obtains an observation-only socket ticket for a joined group. These sockets cannot negotiate media or acquire turns and need no SFU credentials.
- `POST /api/walkie-talkie/connect`: device-only, short-lived single-use media ticket. `GET /api/walkie-talkie/socket/:group?ticket=…` validates Origin, ticket, device authorization, and membership before upgrading.
- `POST /api/walkie-talkie/media`: authorized operations tied to this device's active media connection. SFU session IDs must belong to it; subscriptions resolve approved group publications.

While the widget is open in the foreground it observes joined groups without receiving their audio. A speaking turn in an inactive group shows a Switch action until that turn ends. Switching receives only ongoing live speech; there is no replay. Only one group has SFU media connections. Local pause stops capture and playback; activity observation continues while visible. Hiding the page closes observation and media, and returning restores activity only, requiring local audio activation again.

Group sockets publish member presence and the current speaker. Press requests are granted atomically; the speaker renews every two seconds with a six-second server lease. The client mutes itself if lease acknowledgements stop for five seconds. There is no maximum healthy hold duration. Release, cancellation, focus loss, hidden pages, configuration changes, and disconnects mute locally. Reconnection never resumes a previous talk turn.

Each device establishes a muted publication and up to four receiving connections. Receivers play only the current speaker. This is application-level push-to-talk coordination for authorized clients, not a moderation boundary against a modified client. Only approved group members can negotiate subscriptions through the backend.

SFU operations are serialized by the room coordinator. Outbound provider calls have a ten-second timeout. Each event processes at most one queued media-session cleanup so teardown work cannot hold the room indefinitely; alarms retry and drain the queue up to five attempts per session. Group membership changes take effect in D1 immediately. Media closure is asynchronous and may take additional provider round trips; already buffered audio may still play. Pending cleanup is logged, and management operations surface that state rather than claiming every media track is closed.

Do not log invite codes, device credentials, SFU secrets, SDP, or audio. The feature's operational logs contain only failure classes, provider HTTP status, lease expiry, and pending teardown. Short-lived tickets are not long-lived device tokens; configure production request logging to exclude socket query strings if the logging sink retains full URLs.

## Manual acceptance checks

Do not add automated tests unless requested. Build/type-check the existing apps and perform these checks against an isolated environment:

1. Validate migration 0004 on a disposable copy: memberships, selections and old invites are cleared; group names, owners, authorized devices, and enabled widgets remain. Confirm old sockets lose authorization and cleanup remains retryable.
2. Create an empty group. Enable each phone's widget through its dashboard, generate a code on mobile, preview it as the owner, and confirm admission. Verify no enrollment happens at preview, no credentials leak, nonowners are denied, and first name/photo/device name identify the exact phone.
3. Admit up to five independent phones. Reject a sixth, including concurrent admission. Verify codes are single use under competing group owners, full-group failure preserves the code, and admission preserves another active group.
4. Verify malformed, expired, cancelled, replaced, revoked-device, disabled-widget and consumed codes fail correctly. Retry successful admission, then remove the phone and confirm receipt replay cannot restore membership. Check rate limits and legacy endpoints returning 410.
5. With real phones and configured SFU/TURN credentials, activate audio and verify live playback before release. Exercise Safari and Android Chrome, Wi-Fi and mobile data, simultaneous presses, holds over a minute, release/cancel, focus loss, hidden pages and network changes. Reconnect muted and require a fresh press.
6. Remove/leave/delete/revoke while listening and speaking. Verify new media control is denied, SFU flows close, widget selections refresh, and unresolved closure stays queued for retry.
7. Deny microphone permission, block playback, pause/resume, change panels, and switch groups. Verify inactive-group prompts offer only ongoing live speech, with no replay.
8. Visually inspect dashboard dropdowns and focused panels, mobile code generation/copy/cancel, avatar fallbacks, duplicate profile identities, and existing widgets. Builds and dry runs do not establish physical-phone audio acceptance.

Cloudflare references: [SFU architecture](https://developers.cloudflare.com/realtime/sfu/concepts/architecture/), [connection recipes](https://developers.cloudflare.com/realtime/sfu/get-started/connection-patterns/), [TURN credentials](https://developers.cloudflare.com/realtime/turn/generate-credentials/), and [pricing](https://developers.cloudflare.com/realtime/sfu/platform/pricing/).
