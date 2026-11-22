//! Async `Carbon Protocol v0.1` WebSocket client (`ws://<host>:7420/carbon`) and catalog projector.

pub mod catalog;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use carbon_protocol::{
    ActorRole, BoardStateSnapshot, CarbonCommand, CarbonEvent, CarbonEventEnvelope, ChromePatch,
    ClientActor, ClientMessage, CommandAck, PROTOCOL_VERSION, ServerMessage, SetThemeParams,
    ThemeId, decode_server_message, encode_client_message,
};
use futures_util::{SinkExt, StreamExt};
use memory_core_rs::CourseCatalogSnapshot;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::time::timeout;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::carbon::catalog::project_full_stage_catalog;
use crate::error::{HydrogenError, Result};

/// Handshake metadata captured from Carbon's initial `board.ready` frame upon WebSocket connection.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardReadyInfo {
    pub protocol_version: String,
    pub gpu_backend: String,
    pub viewport_px: [u32; 2],
    pub scale_factor: f32,
}

type PendingMap = Arc<Mutex<BTreeMap<String, oneshot::Sender<ServerMessage>>>>;

/// Live bidirectional WebSocket client connected to a running `carbon-shell` or `carbon-web` host.
pub struct CarbonClient {
    ws_url: String,
    device_id: String,
    board_ready: BoardReadyInfo,
    next_req_id: AtomicU64,
    outbound_tx: mpsc::Sender<String>,
    pending: PendingMap,
    events_rx: Mutex<Option<mpsc::Receiver<CarbonEventEnvelope>>>,
    default_timeout: Duration,
}

impl CarbonClient {
    /// Connect to `ws_url` (e.g. `ws://127.0.0.1:7420/carbon`), await Carbon's `board.ready` handshake,
    /// and spawn lightweight reader/writer tasks for request correlation and live event streaming.
    pub async fn connect(
        ws_url: &str,
        device_id: &str,
        connect_timeout: Duration,
    ) -> Result<Self> {
        let (ws_stream, _) = timeout(connect_timeout, connect_async(ws_url))
            .await
            .map_err(|_| {
                HydrogenError::CarbonTransport(format!(
                    "timed out after {} ms connecting to {ws_url}",
                    connect_timeout.as_millis()
                ))
            })??;

        let (mut sink, mut stream) = ws_stream.split();

        // Wait for Carbon's immediate `board.ready` frame
        let ready_frame = timeout(connect_timeout, stream.next())
            .await
            .map_err(|_| {
                HydrogenError::CarbonTransport(format!(
                    "timed out waiting for board.ready from {ws_url}"
                ))
            })?
            .ok_or_else(|| {
                HydrogenError::CarbonTransport(
                    "Carbon closed WebSocket before sending board.ready".into(),
                )
            })??;

        let ready_text = match ready_frame {
            Message::Text(t) => t.to_string(),
            other => {
                return Err(HydrogenError::CarbonProtocol(format!(
                    "expected UTF-8 text board.ready frame, got {other:?}"
                )));
            }
        };

        let first_msg = decode_server_message(&ready_text)
            .map_err(|e| HydrogenError::CarbonProtocol(format!("{}: {}", e.code_str(), e.message)))?;

        let board_ready = match first_msg {
            ServerMessage::Event(CarbonEventEnvelope {
                event:
                    CarbonEvent::BoardReady {
                        protocol_version,
                        gpu_backend,
                        viewport_px,
                        scale_factor,
                    },
                ..
            }) => BoardReadyInfo {
                protocol_version,
                gpu_backend,
                viewport_px,
                scale_factor,
            },
            other => {
                return Err(HydrogenError::CarbonProtocol(format!(
                    "expected initial board.ready event, received {other:?}"
                )));
            }
        };

        let (outbound_tx, mut outbound_rx) = mpsc::channel::<String>(64);
        let (events_tx, events_rx) = mpsc::channel::<CarbonEventEnvelope>(128);
        let pending: PendingMap = Arc::new(Mutex::new(BTreeMap::new()));
        let pending_reader = Arc::clone(&pending);

        // Spawn writer loop
        tokio::spawn(async move {
            while let Some(raw_json) = outbound_rx.recv().await {
                if sink.send(Message::Text(raw_json.into())).await.is_err() {
                    break;
                }
            }
        });

        // Spawn reader & correlation router loop
        tokio::spawn(async move {
            while let Some(Ok(frame)) = stream.next().await {
                match frame {
                    Message::Text(text) => {
                        let Ok(server_msg) = decode_server_message(text.as_str()) else {
                            continue;
                        };
                        match server_msg {
                            ServerMessage::Event(env) => {
                                let _ = events_tx.try_send(env);
                            }
                            reply => {
                                if let Some(req_id) = extract_request_id(&reply) {
                                    let tx_opt = pending_reader.lock().await.remove(req_id);
                                    if let Some(tx) = tx_opt {
                                        let _ = tx.send(reply);
                                    }
                                }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        });

        Ok(Self {
            ws_url: ws_url.to_string(),
            device_id: device_id.to_string(),
            board_ready,
            next_req_id: AtomicU64::new(1),
            outbound_tx,
            pending,
            events_rx: Mutex::new(Some(events_rx)),
            default_timeout: Duration::from_secs(5),
        })
    }

    /// Connected WebSocket URL.
    pub fn ws_url(&self) -> &str {
        &self.ws_url
    }

    /// Handshake info reported by Carbon's `board.ready` frame.
    pub fn board_ready(&self) -> &BoardReadyInfo {
        &self.board_ready
    }

    /// Take ownership of the asynchronous `CarbonEventEnvelope` stream (`catalog.itemSelected`, `comment.saved`, etc.).
    pub async fn take_event_receiver(&self) -> Option<mpsc::Receiver<CarbonEventEnvelope>> {
        self.events_rx.lock().await.take()
    }

    fn next_id(&self, prefix: &str) -> String {
        let n = self.next_req_id.fetch_add(1, Ordering::Relaxed);
        format!("{prefix}-{n:04}")
    }

    fn actor(&self) -> ClientActor {
        ClientActor {
            student_id: None,
            device_id: Some(self.device_id.clone()),
            role: Some(ActorRole::Driver),
        }
    }

    async fn request_reply(&self, req_id: String, msg: &ClientMessage) -> Result<ServerMessage> {
        let payload = encode_client_message(msg)
            .map_err(|e| HydrogenError::CarbonProtocol(e.message))?;
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(req_id.clone(), tx);

        if self.outbound_tx.send(payload).await.is_err() {
            self.pending.lock().await.remove(&req_id);
            return Err(HydrogenError::CarbonTransport(
                "Carbon WebSocket writer channel closed".into(),
            ));
        }

        let reply = match timeout(self.default_timeout, rx).await {
            Ok(Ok(r)) => r,
            Ok(Err(_)) => {
                return Err(HydrogenError::CarbonTransport(format!(
                    "Carbon closed connection while awaiting reply for '{req_id}'"
                )));
            }
            Err(_) => {
                self.pending.lock().await.remove(&req_id);
                return Err(HydrogenError::CarbonTransport(format!(
                    "timed out awaiting Carbon reply for '{req_id}'"
                )));
            }
        };

        if let ServerMessage::Error { error, .. } = reply {
            return Err(HydrogenError::CarbonProtocol(format!(
                "{:?}: {}",
                error.code, error.message
            )));
        }

        Ok(reply)
    }

    /// Send a `ping` liveness probe and return Carbon's `pong` server timestamp (`ms`).
    pub async fn ping(&self) -> Result<u64> {
        let req_id = self.next_id("ping");
        let msg = ClientMessage::Ping {
            id: Some(req_id.clone()),
            timestamp_ms: Some(now_epoch_ms()),
        };
        match self.request_reply(req_id, &msg).await? {
            ServerMessage::Pong { timestamp_ms, .. } => Ok(timestamp_ms),
            other => Err(HydrogenError::CarbonProtocol(format!(
                "expected pong reply, received {other:?}"
            ))),
        }
    }

    /// Query Carbon's complete `BoardStateSnapshot` (`getState`).
    pub async fn get_state(&self, include_tokens: bool) -> Result<Box<BoardStateSnapshot>> {
        let req_id = self.next_id("state");
        let msg = ClientMessage::GetState {
            version: PROTOCOL_VERSION.to_string(),
            id: Some(req_id.clone()),
            include_tokens,
            include_attachment_data: false,
        };
        match self.request_reply(req_id, &msg).await? {
            ServerMessage::State { snapshot, .. } => Ok(snapshot),
            other => Err(HydrogenError::CarbonProtocol(format!(
                "expected state snapshot reply, received {other:?}"
            ))),
        }
    }

    /// Dispatch a single `CarbonCommand` and await its `CommandAck`.
    pub async fn send_command(&self, command: CarbonCommand) -> Result<CommandAck> {
        let req_id = self.next_id("cmd");
        let msg = ClientMessage::Command {
            version: PROTOCOL_VERSION.to_string(),
            id: Some(req_id.clone()),
            actor: Some(self.actor()),
            timestamp_ms: Some(now_epoch_ms()),
            command,
        };
        match self.request_reply(req_id, &msg).await? {
            ServerMessage::Ack(ack) if ack.ok => Ok(ack),
            ServerMessage::Ack(ack) => Err(HydrogenError::CarbonProtocol(format!(
                "Carbon command rejected: {:?}",
                ack.error
            ))),
            other => Err(HydrogenError::CarbonProtocol(format!(
                "expected command ack, received {other:?}"
            ))),
        }
    }

    /// Dispatch an atomic frame `batch` of `CarbonCommand`s and await `CommandAck`.
    pub async fn send_batch(&self, commands: Vec<CarbonCommand>) -> Result<CommandAck> {
        let req_id = self.next_id("batch");
        let msg = ClientMessage::Batch {
            version: PROTOCOL_VERSION.to_string(),
            id: Some(req_id.clone()),
            actor: Some(self.actor()),
            timestamp_ms: Some(now_epoch_ms()),
            commands,
        };
        match self.request_reply(req_id, &msg).await? {
            ServerMessage::Ack(ack) if ack.ok => Ok(ack),
            ServerMessage::Ack(ack) => Err(HydrogenError::CarbonProtocol(format!(
                "Carbon batch rejected: {:?}",
                ack.error
            ))),
            other => Err(HydrogenError::CarbonProtocol(format!(
                "expected batch ack, received {other:?}"
            ))),
        }
    }

    /// Push the Light, Soft (`WarmPaper`) Theme, Hydrogen Author Mode (`authorMode: true`),
    /// and Full-Stage Course Gallery (`chrome.setCatalog` with embedded `GYR-LOGO-BLUE`)
    /// atomically in a single frame batch.
    pub async fn push_author_catalog(
        &self,
        snap: &CourseCatalogSnapshot,
        brand_logo_data_uri: Option<String>,
    ) -> Result<CommandAck> {
        let catalog_params = project_full_stage_catalog(snap, brand_logo_data_uri);
        let theme_params = SetThemeParams {
            theme: ThemeId::WarmPaper,
            overrides: None,
        };
        let chrome_patch = ChromePatch {
            author_mode: Some(true),
            important_icon: Some(carbon_protocol::ImportantIconStyle::Triangle),
            catalog_button_visible: Some(true),
            ..ChromePatch::default()
        };
        self.send_batch(vec![
            CarbonCommand::SetTheme(theme_params),
            CarbonCommand::ChromeUpdate(chrome_patch),
            CarbonCommand::SetCatalog(Box::new(catalog_params)),
        ])
        .await
    }
}

trait ErrorCodeLabel {
    fn code_str(&self) -> &'static str;
}

impl ErrorCodeLabel for carbon_protocol::ProtocolError {
    fn code_str(&self) -> &'static str {
        match self.code {
            carbon_protocol::ErrorCode::InvalidJson => "invalid_json",
            carbon_protocol::ErrorCode::UnsupportedVersion => "unsupported_version",
            carbon_protocol::ErrorCode::InvalidParameter => "invalid_parameter",
            carbon_protocol::ErrorCode::TargetNotFound => "target_not_found",
            carbon_protocol::ErrorCode::CapabilityNotReady => "capability_not_ready",
            carbon_protocol::ErrorCode::InternalError => "internal_error",
        }
    }
}

fn extract_request_id(msg: &ServerMessage) -> Option<&str> {
    match msg {
        ServerMessage::Ack(ack) => ack.request_id.as_deref(),
        ServerMessage::State { request_id, .. }
        | ServerMessage::Capture { request_id, .. }
        | ServerMessage::Pong { request_id, .. }
        | ServerMessage::Error { request_id, .. } => request_id.as_deref(),
        ServerMessage::Event(_) => None,
    }
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use carbon_protocol::{
        CatalogSelectionKind, CatalogSnapshotSummary, ChromeSnapshot, ChromeVisibility,
        DualTimersSnapshot, ImportantIconStyle, LayoutMode, LayoutSnapshot, LoupeSnapshot,
        NormalizedRect, OpticsSnapshot, ProgressStatus, ReactiveStateSnapshot, ReferenceViewMode,
        ReminderInterval, SplitOrientation, SurfaceSpec, SurfacesSnapshot, ThemeId, TimerMode,
        TimerSnapshot, UserInkSummary, decode_client_message, encode_server_message,
    };
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_async;

    fn sample_state_snapshot(catalog: Option<CatalogSnapshotSummary>) -> BoardStateSnapshot {
        let timer = TimerSnapshot {
            mode: TimerMode::Countdown,
            running: false,
            configured_sec: None,
            elapsed_sec: 0,
            remaining_sec: 0,
            overtime_sec: 0,
            warning_active: false,
            expired: false,
            formatted: "00:00".to_string(),
        };
        let plain_surface = SurfaceSpec::default();
        BoardStateSnapshot {
            protocol_version: PROTOCOL_VERSION.to_string(),
            layout: LayoutSnapshot {
                mode: LayoutMode::Split,
                split_ratio: 0.34,
                left_collapsed: false,
                reference_view: ReferenceViewMode::Question,
                effective_orientation: SplitOrientation::Horizontal,
                left_pane_bounds: Some(NormalizedRect::new(0.0, 0.0, 0.34, 1.0)),
                right_pane_bounds: NormalizedRect::new(0.34, 0.0, 0.66, 1.0),
            },
            theme: ThemeId::WarmPaper,
            surfaces: SurfacesSnapshot {
                left: plain_surface.clone(),
                right: plain_surface,
                detour: None,
            },
            region_space: Vec::new(),
            objects: Vec::new(),
            optics: OpticsSnapshot {
                camera_scale: 1.0,
                camera_center: [0.5, 0.5],
                loupe: LoupeSnapshot {
                    visible: false,
                    x: 0.5,
                    y: 0.5,
                    radius_px: 120.0,
                    magnification: 2.0,
                },
            },
            chrome: ChromeSnapshot {
                mode: ChromeVisibility::Full,
                title: "Gyara+ 11+ Maths".to_string(),
                active_item_id: None,
                paper_badge: None,
                item_label: None,
                marks_chip: None,
                status: ProgressStatus::Default,
                important: false,
                important_icon: ImportantIconStyle::Triangle,
                reminder: ReminderInterval::Off,
                reminder_due: false,
                dictionary_enabled: false,
                dictionary_count: 0,
                catalog_button_visible: true,
                author_mode: true,
                part_tabs: Vec::new(),
                active_part_id: None,
                total_steps: 0,
                active_step: 0,
                active_substep: None,
                breakdown_available: false,
                answer_revealed: false,
                caption_visible: false,
                caption_text: None,
                feedback_status: None,
                feedback: Vec::new(),
                calculator_mode: carbon_protocol::CalculatorMode::NonCalculator,
                step_actions: Vec::new(),
                pinned_references: Vec::new(),
            },
            timers: DualTimersSnapshot {
                session: timer.clone(),
                item: timer,
            },
            detour_depth: 0,
            detour_stack: Vec::new(),
            user_ink: UserInkSummary::default(),
            reactive: ReactiveStateSnapshot::default(),
            media: None,
            catalog,
            telemetry: None,
        }
    }

    #[tokio::test]
    async fn handshakes_pings_pushes_full_stage_catalog_and_streams_events() -> Result<()> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let ws_url = format!("ws://{addr}/carbon");

        // Spawn simulated Carbon Protocol v0.1 server
        let server_task = tokio::spawn(async move {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let Ok(mut ws) = accept_async(tcp).await else {
                return;
            };

            // 1. Immediately emit `board.ready`
            let ready = ServerMessage::Event(CarbonEventEnvelope {
                seq: 1,
                timestamp_ms: 1763838000000,
                actor: None,
                event: CarbonEvent::BoardReady {
                    protocol_version: PROTOCOL_VERSION.to_string(),
                    gpu_backend: "Metal-Mock-120Hz".to_string(),
                    viewport_px: [2560, 1600],
                    scale_factor: 2.0,
                },
            });
            if let Ok(raw) = encode_server_message(&ready) {
                let _ = ws.send(Message::Text(raw.into())).await;
            }

            let mut latest_catalog: Option<CatalogSnapshotSummary> = None;

            while let Some(Ok(Message::Text(txt))) = ws.next().await {
                let Ok(client_msg) = decode_client_message(txt.as_str()) else {
                    continue;
                };
                match client_msg {
                    ClientMessage::Ping { id, .. } => {
                        let pong = ServerMessage::Pong {
                            request_id: id,
                            timestamp_ms: 1763838000123,
                        };
                        if let Ok(raw) = encode_server_message(&pong) {
                            let _ = ws.send(Message::Text(raw.into())).await;
                        }
                    }
                    ClientMessage::Batch { id, commands, .. } => {
                        for cmd in &commands {
                            if let CarbonCommand::SetCatalog(cat) = cmd {
                                latest_catalog = Some(CatalogSnapshotSummary {
                                    open: cat.open,
                                    display_mode: cat.display_mode,
                                    view_level: cat.view_level.unwrap_or_default(),
                                    active_tab: cat.active_tab,
                                    active_subject_id: cat.active_subject_id.clone(),
                                    active_paper_id: cat.active_paper_id.clone(),
                                    active_item_id: cat.active_item_id.clone(),
                                    subject_count: cat.available_subjects.len(),
                                    paper_count: cat.papers.len(),
                                    topic_count: cat.topics.len(),
                                    question_count: cat
                                        .papers
                                        .iter()
                                        .map(|p| p.items.len())
                                        .sum(),
                                });
                            }
                        }
                        let ack = ServerMessage::Ack(CommandAck {
                            request_id: id,
                            ok: true,
                            action_ids: vec!["act-1".to_string(), "act-2".to_string()],
                            error: None,
                        });
                        if let Ok(raw) = encode_server_message(&ack) {
                            let _ = ws.send(Message::Text(raw.into())).await;
                        }

                        // Emit a simulated user click on the catalog!
                        let click_ev = ServerMessage::Event(CarbonEventEnvelope {
                            seq: 2,
                            timestamp_ms: 1763838000250,
                            actor: None,
                            event: CarbonEvent::CatalogItemSelected {
                                kind: CatalogSelectionKind::Topic,
                                item_id: "GYR-MTH-T-ALG".to_string(),
                                subject_id: Some("GYR-MTH".to_string()),
                                paper_id: None,
                                topic_id: Some("GYR-MTH-T-ALG".to_string()),
                            },
                        });
                        if let Ok(raw) = encode_server_message(&click_ev) {
                            let _ = ws.send(Message::Text(raw.into())).await;
                        }
                    }
                    ClientMessage::GetState { id, .. } => {
                        let state = ServerMessage::State {
                            request_id: id,
                            snapshot: Box::new(sample_state_snapshot(latest_catalog.clone())),
                        };
                        if let Ok(raw) = encode_server_message(&state) {
                            let _ = ws.send(Message::Text(raw.into())).await;
                        }
                    }
                    _ => {}
                }
            }
        });

        let client = CarbonClient::connect(
            &ws_url,
            "hydrogen-test-driver",
            Duration::from_secs(2),
        )
        .await?;
        assert_eq!(client.board_ready().protocol_version, "0.1");
        assert_eq!(client.board_ready().gpu_backend, "Metal-Mock-120Hz");

        let pong_ts = client.ping().await?;
        assert_eq!(pong_ts, 1763838000123);

        let empty_snap = CourseCatalogSnapshot {
            meta: None,
            books: Vec::new(),
            chapters: Vec::new(),
            papers: Vec::new(),
            paper_sections: Vec::new(),
            topics: Vec::new(),
            topic_cards: Vec::new(),
            question_types: Vec::new(),
            questions: Vec::new(),
            open_comment_counts: BTreeMap::new(),
            total_open_comments: 0,
        };
        let ack = client
            .push_author_catalog(
                &empty_snap,
                Some("data:image/png;base64,iVBORw0KGgo=".to_string()),
            )
            .await?;
        assert!(ack.ok);

        let state = client.get_state(false).await?;
        assert_eq!(
            state.catalog.as_ref().map(|c| c.subject_count),
            Some(4),
            "Gyara+ catalog should register 4 jewel-toned subject hero cards"
        );

        let mut events_rx = client.take_event_receiver().await.ok_or_else(|| {
            HydrogenError::Validation("expected event receiver to be available".into())
        })?;
        let received = timeout(Duration::from_secs(2), events_rx.recv())
            .await
            .map_err(|_| HydrogenError::CarbonTransport("event timeout".into()))?
            .ok_or_else(|| HydrogenError::CarbonTransport("event channel closed".into()))?;
        assert_eq!(received.event.event_name(), "catalog.itemSelected");

        server_task.abort();
        Ok(())
    }
}
