# Caller setup and validation

Groups are owned by accounts and start empty. Every phone is already authorized to its own account and has the Caller widget enabled through that account's dashboard. The phone generates a single-use device code inside the widget; the user shares it with the group owner, who previews the device and confirms admission through Group settings beneath the group name.

A group has at most five joined devices. Device memberships are independent and persist while offline or paused. A phone may join multiple groups but uses one active group. Admission preserves an existing active selection; the first membership becomes active. Microphone capture and call playback require an explicit start, pickup, or join action. Mobile shows profile first names and photos with avatar fallbacks; dashboard management also shows device names.

## Configure Cloudflare

1. Create two Cloudflare Realtime SFU applications: development and production. Do not reuse one application's credentials across environments.
2. Supply `WALKIE_SFU_APP_ID` and `WALKIE_SFU_APP_SECRET` on the backend. Local development can use ignored `backend/.dev.vars`. For production, put the app ID in `[env.production.vars]` of `backend/wrangler.toml` and install the secret with `wrangler secret put WALKIE_SFU_APP_SECRET --env production`, from `backend/`.
3. For networks requiring a TURN relay, create a Cloudflare TURN key. Configure `WALKIE_TURN_KEY_ID` and secret `WALKIE_TURN_API_TOKEN` on the backend. Without that configuration the browser uses Cloudflare STUN; restricted networks may not connect. Production should configure TURN. The browser receives only generated, one-hour credentials. Each reconnect fetches fresh credentials.
4. Apply D1 migrations locally with `wrangler d1 migrations apply dashboard --local`, from `backend/`. Apply production migrations through the repository's existing production migration command. Migration 0003 introduced device membership. Migration 0004 intentionally clears every walkie membership, active selection, old group invite, and widget group identifier, while preserving groups, ownership, application authorization, and widget enablement. Back up the database before applying this agreed reset; `make start-api` and `make db-schema-local` now apply migrations as well as the baseline schema, so an existing local database receives new feature tables automatically.
5. Build the Worker from `backend/` with `worker-build --release --locked`. Durable Object bindings are declared in both environments: `WALKIE_GROUP` (migration `v4`), `CLIENT_GATEWAY` (migration `v5`, the only client WebSocket), and migration `v6` deletes the retired `DeviceObject`/`DevicesHubObject` classes whose channels moved onto the gateway.
6. Deploy the backend before the frontend and mobile builds. Frontend and mobile keep their existing `VITE_API_URL` settings and origins. Microphone access on phones requires HTTPS.

No audio storage, recording service, transcription service, or external media server is used. Cloudflare terminates standard WebRTC transport encryption at the SFU; this is not additional end-to-end group encryption.

## Behavior and interfaces

- `GET /api/walkie-talkie/groups?device_token=…`: dashboard-scoped memberships plus account-owned groups, public device ID, joined/active flags, names/photos, and device presence. Authentication credentials are never returned in rosters or previews.
- `POST /api/walkie-talkie/groups`: `{name}` creates an empty group owned by the account. No phone is enrolled or selected automatically.
- `PATCH/DELETE /api/walkie-talkie/groups/:group`: owner rename/delete. Deletion unregisters every group device, cleans up active selections (disabling the walkie widget for devices left with 0 groups), closes participation, and queues SFU cleanup. Device authorization and other memberships remain intact.
- `GET/POST/DELETE /api/walkie-talkie/device-code`: device authentication only; retrieve, generate/replace, or cancel this enabled phone's code. Responses contain `{device_code:{code,expires_at}|null,status}` with status `active`, `used`, `expired`, or `none`. Codes are six digits, single use, valid for ten minutes, collision-checked and rate-limited. Polled by mobile only while pairing sheet is open and an active code exists.
- `POST /api/walkie-talkie/groups/:group/device-preview`: owner-only `{code}` returns `{device,group_id,group_name,member_count}`. The public device preview contains first name, photo, device name, and public device ID. Preview never reserves or consumes the code.
- `POST /api/walkie-talkie/groups/:group/devices`: owner-only `{code,device_id}` confirms the previewed device. A D1 transaction revalidates owner, code, exact device, authorization, enabled widget, and five-device capacity; admits membership, establishes only the first active selection, and consumes the code. Full groups roll back without consuming codes or changing selection. Receipt retries require the same surviving membership in the same group; consumed codes cannot recreate removed membership or admit another group.
- Legacy `POST /join` and `POST/DELETE /groups/:group/invite` return 410, explaining the phone-generated code flow.
- `DELETE /api/walkie-talkie/groups/:group/members/:device_id`: owner removes a phone; an account may make its own device leave. Ownership survives the owner's phones leaving. If the removed device has no remaining groups, its Caller widget is automatically disabled and pushed via WebSocket.
- `POST /api/walkie-talkie/active`: `{group_id,device_token}` for dashboard sessions, `{group_id}` for authenticated phones. Only joined groups can be activated. Idle group selection updates the saved widget configuration; selecting a different group is disabled during a call. Answering an incoming call selects its group locally.
- Existing full-set widget PUT stores `{widget_key:"walkie_talkie",enabled:true,config:{group_id}}`. Selection requires membership. Disablement ends live participation without removing memberships.
- `POST /api/walkie-talkie/memberships`: this phone's memberships and active selection. Synchronized via real-time push (`widgets_updated` on the mux socket), tab visibility changes, and a 60s fallback poll.
- `POST /api/walkie-talkie/media`: version-2 operations include `{group_id,protocol_version:2,connection_id,operation,...}`. The device must belong to the group and own its media connection. SFU session IDs must belong to it; receive subscriptions require both connections to participate in the same call.
- `POST /api/walkie-talkie/call-state`: `{group_id}` revalidates membership and returns the current call for answer/switch preflight.

## Single mux socket (all realtime)

Each client holds exactly **one** WebSocket, owned by `ClientGatewayObject` (one instance per client: `device:<token>`, `user:<id>`, or `pairing:<session>`). No other Durable Object accepts client sockets: `PairingObject` is a state referee and `WalkieGroupObject` a per-group coordinator, both reached by internal fetches. New realtime features add a channel, never a socket.

- `GET /api/live/device/:token` — phone socket (device token validated against D1). Channels: `device`, `walkie`.
- `GET /api/live/pairing/:session_id` — pre-authorization phone socket. Channel: `pairing` only.
- `GET /api/live` — dashboard socket (HttpOnly session cookie + Origin check). Channel: `hub`.

Every frame is an envelope `{v:1,channel,group_id?,...payload}`. Caller subscriptions use `{channel:"walkie",type:"subscribe",protocol_version:2,group_id,observe}`. Observation never captures a microphone; it persists across right-bar pager changes and covers every joined group. One 10-second socket heartbeat refreshes every subscription. Page closure stops observation and media; background, locked-phone, and closed-page incoming delivery are not supported guarantees.

Caller uses one connected call per device and one owning browser tab. The device gateway reserves ownership across groups before opening media, publishes `caller_ownership` updates, and acknowledges media unsubscribe only after leaving and releasing ownership. Other tabs show an in-use state. Generation identifiers prevent delayed unsubscribe/close operations from releasing a newer reservation.

### Calls

The group room stores `{call_id,started_by,participants,started_at,ringing_until,established,declined}`. `started_by` is the stable public device ID; participants are current media connection IDs. Each group has at most one call. Invitations last 60 seconds from creation, including for unanswered members after the first pickup. An unanswered call expires at the deadline; an established call remains joinable until it ends. The server alarm schedules against the unanswered-call deadline.

All actions require protocol version 2. `call_start` creates a call after publication is ready. `call_join`, `call_decline`, `call_leave`, `call_cancel`, and `call_end` require the exact `call_id`; stale messages cannot affect a replacement call. Only explicit user actions establish media. Legacy subscriptions get a refresh-required error; PTT signals and armed-device automatic joining are unsupported.

- Start or pick up with the microphone on; participants can mute themselves. All participants hear each other.
- Decline silences only that device for the call. It can deliberately join later, as can members who missed ringing.
- Leave removes only that participant. After a call has connected at least two people, it ends when fewer than two remain. A detected disconnect removes the participant without a recovery grace period. Departures by observation-only clients do not end a call.
- Only the original starter, while participating, can end the call for everyone. That action is separate from Leave and acts immediately without confirmation. The starter may leave, then regain the action by rejoining; authority never transfers. Before pickup, the starter can cancel the outgoing call.

### Widget and call waiting

A new incoming call selects the Caller page once; users remain free to navigate back to Stocks or other widgets. Calls and microphone ownership outlive widget unmounts. Pending calls appear together in a compact list. During a connected call they appear in one call-waiting modal with Answer and switch and Decline. Answer and switch checks microphone readiness and current destination call identity before leaving, then revalidates on join. If connection fails after departure, a previous call that still exists is offered for manual rejoin. There is no automatic rejoin and no simultaneous media in two calls.

A local synthesized ringtone accompanies incoming calls, with a short call-waiting alert. Browser audio restrictions never suppress visual invitations; Enable ringtone/audio provides a user gesture to resume sound. Answer, decline, timeout, cancellation, and end stop the applicable alert. Closing the waiting modal preserves the current conversation.

### Compatibility and media cleanup

The public widget key `walkie_talkie` and `/api/walkie-talkie` routes remain for compatibility with saved configuration. Version-2 media requests explicitly identify their authorized group rather than depending on an asynchronously changing saved selection. Caller adds no D1 migration and performs no membership reset. On first load of legacy Durable Object room state, old transient connections, calls, and PTT state are discarded and their media is queued for cleanup; membership and configuration remain in D1.

Room changes are serialized. Media negotiation reserves a session under a short state lock, calls the SFU outside that lock, and revalidates the reservation and call participation before committing receiving media. Leaving, ending, expiry, access changes, and disconnects retire affected media into durable cleanup. Provider calls have bounded timeouts; alarms drain cleanup outside the state lock and retain in-flight work for retry. Media closure is asynchronous and already buffered audio may still play. The browser stops capture/playback locally on departure. Builds alone do not prove physical-phone audio behavior.

Do not log device codes, credentials, SFU secrets, SDP, or audio. Keep operational diagnostics limited to failure classes and pending teardown.

## Manual acceptance checks

Do not add automated tests unless requested. Build/type-check the existing apps and perform these checks against an isolated environment:

1. Validate migration 0004 on a disposable copy: memberships, selections and old invites are cleared; group names, owners, authorized devices, and enabled widgets remain. Confirm old sockets lose authorization and cleanup remains retryable.
2. Create an empty group. Enable each phone's widget through its dashboard, generate a code on mobile, preview it as the owner, and confirm admission. Verify no enrollment happens at preview, no credentials leak, nonowners are denied, and first name/photo/device name identify the exact phone.
3. Admit up to five independent phones. Reject a sixth, including concurrent admission. Verify codes are single use under competing group owners, full-group failure preserves the code, and admission preserves another active group.
4. Verify malformed, expired, cancelled, replaced, revoked-device, disabled-widget and consumed codes fail correctly. Retry successful admission, then remove the phone and confirm receipt replay cannot restore membership. Check rate limits and legacy endpoints returning 410.
5. With two or more real phones and configured SFU/TURN, start a call and verify no microphone capture or call audio on recipients before explicit pickup. Check bidirectional audio, independent mute, 60-second ringing after another member answers, decline, late join, and unanswered expiry.
6. With three participants, leave individually and continue with two. Let the starter leave and rejoin, verifying authority restoration. Verify nonstarters cannot end everyone; starter End for everyone acts immediately. End an established call when one participant remains. Unrelated observer/tab departures must not cancel ringing.
7. Move between Caller and Stocks while ringing/talking. Confirm one automatic page selection per new incoming call, free subsequent navigation, and no media loss. Inspect 360×640 and 390×844 viewports within the retained rotation, including long group names and multiple invitations.
8. Call another joined group during a conversation. Check one waiting modal, local decline, answer-and-switch, destination expiry, denied microphone access, manual rejoin after a failed switch, and no duplicate capture. Open duplicate tabs and verify only one can own media.
9. Disconnect, revoke access, disable the widget, or remove membership while connected. Verify immediate local teardown upon detection, server cleanup, no automatic rejoin, and errors for stale call IDs, obsolete clients, and unauthorized ending/subscription actions. Check ringtone/playback restrictions and visible recovery controls.

Cloudflare references: [SFU architecture](https://developers.cloudflare.com/realtime/sfu/concepts/architecture/), [connection recipes](https://developers.cloudflare.com/realtime/sfu/get-started/connection-patterns/), [TURN credentials](https://developers.cloudflare.com/realtime/turn/generate-credentials/), and [pricing](https://developers.cloudflare.com/realtime/sfu/platform/pricing/).
