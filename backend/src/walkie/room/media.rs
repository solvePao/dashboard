//! Provider negotiation and durable cleanup, outside the room event gate.
use super::*;

impl WalkieGroupObject {
    pub(super) async fn coordinate<T: 'static>(
        &self,
        operation: impl std::future::Future<Output = AppResult<T>> + 'static,
    ) -> AppResult<T> {
        self.state
            .block_concurrency_while(async move { Ok(operation.await) })
            .await
            .map_err(db_error)?
    }

    async fn update_media<T: 'static>(
        &self,
        id: String,
        token: String,
        broadcast: bool,
        update: impl FnOnce(&mut Room) -> AppResult<T> + 'static,
    ) -> AppResult<T> {
        let coordinator = self.clone();
        self.coordinate(async move {
            let mut room = coordinator.load().await.map_err(db_error)?;
            if !room
                .clients
                .get(&id)
                .is_some_and(|c| c.token == token && !c.observe)
            {
                return Err(AppError::forbidden("Audio session ended."));
            }
            let value = update(&mut room)?;
            coordinator.save(&room).await.map_err(db_error)?;
            if broadcast {
                coordinator.broadcast(&room).await.map_err(db_error)?;
            }
            Ok(value)
        })
        .await
    }

    pub(super) async fn drain_cleanup(&self) -> AppResult<()> {
        let coordinator = self.clone();
        let pending = self
            .coordinate(async move {
                let mut room = coordinator.load().await.map_err(db_error)?;
                if room.cleanup_in_flight.is_some() && room.cleanup_started_at + 30_000 > now() {
                    return Ok(None);
                }
                let session = room.cleanup_in_flight.take().or_else(|| {
                    if room.cleanup.is_empty() {
                        None
                    } else {
                        Some(room.cleanup.remove(0))
                    }
                });
                let started = now();
                room.cleanup_in_flight = session.clone();
                room.cleanup_started_at = started;
                coordinator.save(&room).await.map_err(db_error)?;
                Ok(session.map(|session| (session, started)))
            })
            .await?;
        let Some((mut session, started)) = pending else {
            return Ok(());
        };
        let sfu = SfuService(&self.env);
        // Persist the in-flight item across provider I/O so interruption cannot lose teardown.
        let result = if session.pending {
            Ok(())
        } else {
            match sfu
                .request(Method::Get, &format!("/sessions/{}", session.id), None)
                .await
            {
                Ok(value) => {
                    session.mids = value
                        .get("tracks")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter(|t| t.get("status").and_then(Value::as_str) != Some("inactive"))
                        .filter_map(|t| t.get("mid").and_then(Value::as_str).map(str::to_string))
                        .collect();
                    sfu.close(&session.id, &session.mids).await
                }
                Err(error) if [404, 410, 425].contains(&error.status) => Ok(()),
                Err(error) => Err(error),
            }
        };
        let coordinator = self.clone();
        self.coordinate(async move {
            let mut room = coordinator.load().await.map_err(db_error)?;
            if room.cleanup_started_at != started
                || room.cleanup_in_flight.as_ref().map(|s| s.id.as_str())
                    != Some(session.id.as_str())
            {
                return Ok(());
            }
            room.cleanup_in_flight = None;
            if result.is_err() {
                session.attempts += 1;
                if session.attempts < 5 {
                    console_error!("walkie media teardown pending retry");
                    room.cleanup.push(session);
                } else {
                    console_error!("walkie media teardown dropped after max retries");
                }
            }
            coordinator.save(&room).await.map_err(db_error)
        })
        .await
    }

    pub(super) async fn media(&self, room: &mut Room, id: &str, data: &Value) -> AppResult<Value> {
        let sfu = SfuService(&self.env);
        let op = data.get("operation").and_then(Value::as_str).unwrap_or("");
        let token = room
            .clients
            .get(id)
            .ok_or_else(|| AppError::forbidden("Audio session ended."))?
            .token
            .clone();
        if op == "ice" {
            return sfu.ice_servers().await.map(|v| json!({"ice_servers":v}));
        }
        if op == "ready" {
            let connection = id.to_string();
            return self
                .update_media(id.into(), token, true, move |room| {
                    let client = room
                        .clients
                        .get_mut(&connection)
                        .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                    if !client
                        .sessions
                        .iter()
                        .any(|s| s.target.is_none() && !s.pending && !s.mids.is_empty())
                    {
                        return Err(AppError::bad_request("Publish audio first."));
                    }
                    client.published = true;
                    Ok(json!({"ok":true}))
                })
                .await;
        }
        if op == "close" {
            let target = data
                .get("target")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::bad_request("Missing publication."))?
                .to_string();
            let connection = id.to_string();
            return self
                .update_media(id.into(), token, false, move |room| {
                    let client = room
                        .clients
                        .get_mut(&connection)
                        .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                    let mut kept = Vec::new();
                    for session in std::mem::take(&mut client.sessions) {
                        if session.target.as_deref() == Some(target.as_str()) {
                            room.cleanup.push(session);
                        } else {
                            kept.push(session);
                        }
                    }
                    client.sessions = kept;
                    Ok(json!({"ok":true}))
                })
                .await;
        }
        if op == "publish" || op == "subscribe" {
            let requested_target = if op == "subscribe" {
                Some(
                    data.get("target")
                        .and_then(Value::as_str)
                        .ok_or_else(|| AppError::bad_request("Missing publication."))?
                        .to_string(),
                )
            } else {
                None
            };
            let subscription_target = requested_target.clone();
            let subscription_call = if requested_target.is_some() { room.call.as_ref().map(|c| c.call_id.clone()) } else { None };
            let local_offer = if op == "publish" {
                let mid = data
                    .get("mid")
                    .and_then(Value::as_str)
                    .filter(|s| s.len() < 16)
                    .ok_or_else(|| AppError::bad_request("Missing audio track."))?;
                let sdp = data
                    .get("sdp")
                    .and_then(Value::as_str)
                    .filter(|s| s.len() < 100_000)
                    .ok_or_else(|| AppError::bad_request("Invalid offer."))?;
                Some(
                    json!({"sessionDescription":{"type":"offer","sdp":sdp},"tracks":[{"location":"local","mid":mid,"trackName":"audio"}]}),
                )
            } else {
                None
            };
            let reservation = random_id()?;
            let reserved = reservation.clone();
            let connection = id.to_string();
            // Reserve under a short gate; SFU requests must not block heartbeats.
            let target = self
                .update_media(id.into(), token.clone(), false, move |room| {
                    let target = if let Some(target) = requested_target {
                        if !room.call.as_ref().is_some_and(|call|
                            call.participants.contains(&connection) && call.participants.contains(&target)) {
                            return Err(AppError::forbidden("Join this call before subscribing to audio."));
                        }
                        if target == connection {
                            return Err(AppError::bad_request("Cannot subscribe to yourself."));
                        }
                        let publisher = room
                            .clients
                            .get(&target)
                            .filter(|c| c.published)
                            .ok_or_else(|| AppError::not_found("Participant audio is unavailable."))?;
                        let session = publisher
                            .sessions
                            .iter()
                            .find(|s| s.target.is_none() && !s.pending)
                            .ok_or_else(|| AppError::not_found("Audio is not ready."))?;
                        Some((target, session.id.clone()))
                    } else {
                        None
                    };
                    let client = room
                        .clients
                        .get_mut(&connection)
                        .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                    if client.sessions.len() >= 5
                        || client
                            .sessions
                            .iter()
                            .any(|s| s.target.as_deref() == target.as_ref().map(|t| t.0.as_str()))
                    {
                        return Err(AppError::new(
                            "Audio session already exists. Reconnect audio.",
                            409,
                        ));
                    }
                    client.sessions.push(MediaSession {
                        id: reserved,
                        pending: true,
                        mids: Vec::new(),
                        target: target.as_ref().map(|t| t.0.clone()),
                        attempts: 0,
                    });
                    Ok(target)
                })
                .await?;
            let created = match sfu.request(Method::Post, "/sessions/new", None).await {
                Ok(created) => created,
                Err(error) => {
                    let connection = id.to_string();
                    let _ = self
                        .update_media(id.into(), token, false, move |room| {
                            let client = room.clients.get_mut(&connection).ok_or_else(|| {
                                AppError::forbidden("Audio session ended.")
                            })?;
                            client.sessions.retain(|s| s.id != reservation);
                            Ok(())
                        })
                        .await;
                    return Err(error);
                }
            };
            let session = created
                .get("sessionId")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::bad_gateway("Invalid audio session."))?
                .to_string();
            let actual = session.clone();
            let connection = id.to_string();
            let recorded = self.update_media(id.into(), token.clone(), false, move |room| {
                let reserved = room
                    .clients
                    .get_mut(&connection)
                    .ok_or_else(|| AppError::forbidden("Audio session ended."))?
                    .sessions
                    .iter_mut()
                    .find(|s| s.id == reservation)
                    .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                reserved.id = actual;
                reserved.pending = false;
                Ok(())
            })
            .await;
            if let Err(error) = recorded {
                // The participant may leave while /sessions/new is in flight.
                // The returned provider session still needs durable teardown.
                let coordinator = self.clone();
                let abandoned = MediaSession { id: session, pending: false, mids: Vec::new(), target: None, attempts: 0 };
                self.coordinate(async move {
                    let mut room = coordinator.load().await.map_err(db_error)?;
                    room.cleanup.push(abandoned);
                    coordinator.save(&room).await.map_err(db_error)
                }).await?;
                return Err(error);
            }
            let request = if let Some((_, publisher)) = target {
                json!({"tracks":[{"location":"remote","sessionId":publisher,"trackName":"audio"}]})
            } else if let Some(offer) = local_offer {
                offer
            } else {
                return Err(AppError::bad_request("Missing audio track."));
            };
            let mut result = match sfu
                .request(
                    Method::Post,
                    &format!("/sessions/{session}/tracks/new"),
                    Some(request),
                )
                .await
            {
                Ok(result) => result,
                Err(error) => {
                    // A failed provider response may still have created tracks after earlier teardown.
                    let coordinator = self.clone();
                    let abandoned = MediaSession {
                        id: session,
                        pending: false,
                        mids: Vec::new(),
                        target: None,
                        attempts: 0,
                    };
                    let connection = id.to_string();
                    self.coordinate(async move {
                        let mut room = coordinator.load().await.map_err(db_error)?;
                        if let Some(client) = room.clients.get_mut(&connection) {
                            client.sessions.retain(|stored| stored.id != abandoned.id);
                        }
                        room.cleanup.push(abandoned);
                        coordinator.save(&room).await.map_err(db_error)
                    })
                    .await?;
                    return Err(error);
                }
            };
            let mids: Vec<String> = result
                .get("tracks")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|t| t.get("mid").and_then(Value::as_str).map(str::to_string))
                .collect();
            let actual = session.clone();
            let committed_mids = mids.clone();
            let connection = id.to_string();
            let committed = self
                .update_media(id.into(), token, false, move |room| {
                    if let Some(target) = subscription_target {
                        if !room.call.as_ref().is_some_and(|call| Some(&call.call_id) == subscription_call.as_ref()
                            && call.participants.contains(&connection) && call.participants.contains(&target)) {
                            return Err(AppError::forbidden("This call has ended."));
                        }
                    }
                    let stored = room
                        .clients
                        .get_mut(&connection)
                        .ok_or_else(|| AppError::forbidden("Audio session ended."))?
                        .sessions
                        .iter_mut()
                        .find(|s| s.id == actual)
                        .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                    stored.mids = committed_mids;
                    Ok(())
                })
                .await;
            if let Err(error) = committed {
                // Invalidation can race provider negotiation. Queue the resulting tracks for closure.
                let coordinator = self.clone();
                let abandoned = MediaSession {
                    id: session,
                    pending: false,
                    mids,
                    target: None,
                    attempts: 0,
                };
                self.coordinate(async move {
                    let mut room = coordinator.load().await.map_err(db_error)?;
                    room.cleanup.push(abandoned);
                    coordinator.save(&room).await.map_err(db_error)
                })
                .await?;
                return Err(error);
            }
            result["session_id"] = json!(session);
            return Ok(result);
        }
        if op == "answer" {
            let session = data.get("session_id").and_then(Value::as_str).unwrap_or("");
            let client = room
                .clients
                .get(id)
                .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
            if !client
                .sessions
                .iter()
                .any(|s| s.id == session && !s.pending)
            {
                return Err(AppError::forbidden(
                    "Session does not belong to this connection.",
                ));
            }
            let sdp = data
                .get("sdp")
                .and_then(Value::as_str)
                .filter(|s| s.len() < 100_000)
                .ok_or_else(|| AppError::bad_request("Invalid answer."))?;
            return sfu
                .request(
                    Method::Put,
                    &format!("/sessions/{session}/renegotiate"),
                    Some(json!({"sessionDescription":{"type":"answer","sdp":sdp}})),
                )
                .await;
        }
        Err(AppError::bad_request("Unknown audio operation."))
    }
}
