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
                let Some(session) = session else {
                    return Ok(None);
                };
                let started = now();
                room.cleanup_in_flight = Some(session.clone());
                room.cleanup_started_at = started;
                coordinator.save(&room).await.map_err(db_error)?;
                Ok(Some((session, started)))
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

    async fn retire_failed_session(
        &self,
        connection: &str,
        session: String,
        pending: bool,
    ) -> AppResult<()> {
        let coordinator = self.clone();
        let connection = connection.to_string();
        self.coordinate(async move {
            let mut room = coordinator.load().await.map_err(db_error)?;
            if let Some(client) = room.clients.get_mut(&connection) {
                client.sessions.retain(|stored| stored.id != session);
            }
            if !pending && !room.cleanup.iter().any(|stored| stored.id == session) {
                room.cleanup.push(MediaSession {
                    id: session,
                    pending: false,
                    prepared: false,
                    mids: Vec::new(),
                    target: None,
                    attempts: 0,
                });
            }
            coordinator.save(&room).await.map_err(db_error)
        })
        .await
    }

    async fn open_media_session(
        &self,
        id: &str,
        token: &str,
        subscription: Option<(String, String)>,
        prepared: bool,
    ) -> AppResult<(String, Option<String>)> {
        let reservation = random_id()?;
        let reserved = reservation.clone();
        let connection = id.to_string();
        // Reserve under a short gate and release it before calling the provider.
        let publisher = self
            .update_media(id.into(), token.into(), false, move |room| {
                let publisher = if let Some((target, call_id)) = &subscription {
                    if target == &connection {
                        return Err(AppError::bad_request("Cannot subscribe to yourself."));
                    }
                    if !room.call.as_ref().is_some_and(|call| {
                        call.call_id == *call_id
                            && call.participants.contains(&connection)
                            && call.participants.contains(target)
                    }) {
                        return Err(AppError::forbidden(
                            "Join this call before subscribing to audio.",
                        ));
                    }
                    let session = room
                        .clients
                        .get(target)
                        .filter(|client| client.published)
                        .and_then(|client| {
                            client
                                .sessions
                                .iter()
                                .find(|s| s.target.is_none() && !s.mids.is_empty())
                        })
                        .ok_or_else(|| AppError::not_found("Participant audio is unavailable."))?;
                    Some(session.id.clone())
                } else {
                    None
                };
                let target = subscription.map(|(target, _)| target);
                let client = room
                    .clients
                    .get_mut(&connection)
                    .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                if client.sessions.len() >= 5 || client.sessions.iter().any(|s| s.target == target)
                {
                    return Err(AppError::new(
                        "Audio session already exists. Reconnect audio.",
                        409,
                    ));
                }
                client.sessions.push(MediaSession {
                    id: reserved,
                    pending: true,
                    prepared,
                    mids: Vec::new(),
                    target,
                    attempts: 0,
                });
                Ok(publisher)
            })
            .await?;

        let sfu = SfuService(&self.env);
        let created = sfu
            .request(Method::Post, "/sessions/new", None)
            .await
            .and_then(|value| {
                value
                    .get("sessionId")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| AppError::bad_gateway("Invalid audio session."))
            });
        let session = match created {
            Ok(session) => session,
            Err(error) => {
                self.retire_failed_session(id, reservation, true).await?;
                return Err(error);
            }
        };
        let actual = session.clone();
        let connection = id.to_string();
        let recorded = self
            .update_media(id.into(), token.into(), false, move |room| {
                let reserved = room
                    .clients
                    .get_mut(&connection)
                    .and_then(|client| client.sessions.iter_mut().find(|s| s.id == reservation))
                    .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                reserved.id = actual;
                reserved.pending = false;
                Ok(())
            })
            .await;
        if let Err(error) = recorded {
            // The participant may leave while the provider creates the session.
            self.retire_failed_session(id, session, false).await?;
            return Err(error);
        }
        Ok((session, publisher))
    }

    async fn claim_prepared_publisher(
        &self,
        id: &str,
        token: &str,
        session: &str,
    ) -> AppResult<()> {
        let connection = id.to_string();
        let session = session.to_string();
        self.update_media(id.into(), token.into(), false, move |room| {
            let prepared =
                room.clients
                    .get_mut(&connection)
                    .and_then(|client| {
                        client.sessions.iter_mut().find(|s| {
                            s.id == session && s.target.is_none() && s.prepared && !s.pending
                        })
                    })
                    .ok_or_else(|| AppError::forbidden("Prepared audio session is unavailable."))?;
            prepared.prepared = false;
            Ok(())
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
        if op == "prepare" {
            let (opened, ice) = futures_util::join!(
                self.open_media_session(id, &token, None, true),
                sfu.ice_servers(),
            );
            let (session, _) = opened?;
            return match ice {
                Ok(servers) => Ok(json!({"session_id":session,"ice_servers":servers})),
                Err(error) => {
                    self.retire_failed_session(id, session, false).await?;
                    Err(error)
                }
            };
        }
        if op == "ready" {
            let connection = id.to_string();
            return self
                .update_media(id.into(), token, true, move |room| {
                    let client = room
                        .clients
                        .get_mut(&connection)
                        .ok_or_else(|| AppError::forbidden("Audio session ended."))?;
                    if !client.has_publication() {
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
            let subscription = if op == "subscribe" {
                let target = data
                    .get("target")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AppError::bad_request("Missing publication."))?;
                let call_id = data
                    .get("call_id")
                    .and_then(Value::as_str)
                    .or_else(|| room.call.as_ref().map(|call| call.call_id.as_str()))
                    .ok_or_else(|| AppError::forbidden("This call has ended."))?;
                Some((target.to_string(), call_id.to_string()))
            } else {
                None
            };
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
            let (session, request) = if let Some(offer) = local_offer {
                let session = if let Some(prepared) = data.get("session_id").and_then(Value::as_str)
                {
                    self.claim_prepared_publisher(id, &token, prepared).await?;
                    prepared.to_string()
                } else {
                    self.open_media_session(id, &token, None, false).await?.0
                };
                (session, offer)
            } else {
                let (session, publisher) = self
                    .open_media_session(id, &token, subscription.clone(), false)
                    .await?;
                (
                    session,
                    json!({"tracks":[{"location":"remote","sessionId":publisher,"trackName":"audio"}]}),
                )
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
                    self.retire_failed_session(id, session, false).await?;
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
            let connection = id.to_string();
            let committed = self
                .update_media(id.into(), token, false, move |room| {
                    if let Some((target, call_id)) = subscription {
                        if !room.call.as_ref().is_some_and(|call| {
                            call.call_id == call_id
                                && call.participants.contains(&connection)
                                && call.participants.contains(&target)
                        }) {
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
                    stored.mids = mids;
                    Ok(())
                })
                .await;
            if let Err(error) = committed {
                // Tracks can be created after a departure already queued cleanup.
                self.retire_failed_session(id, session, false).await?;
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
            if !client.sessions.iter().any(|s| {
                s.id == session
                    && !s.pending
                    && !s.mids.is_empty()
                    && s.target.as_ref().is_some_and(|target| {
                        room.call.as_ref().is_some_and(|call| {
                            call.participants
                                .iter()
                                .any(|participant| participant == id)
                                && call.participants.contains(target)
                        })
                    })
            }) {
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
