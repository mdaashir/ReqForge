//! WebSocket sync endpoint using y-sync v1 protocol.
//!
//! yrs v0.27 uses `Rc<RefCell<...>>` internally, making `Doc` !Send.
//! To work around this, each WebSocket connection is handled on a
//! dedicated OS thread running its own single-threaded tokio runtime.
//! The thread's handle is sent to axum's `on_upgrade` which bridges
//! the Send boundary.
//!
//! Messages from other peers in the same document room arrive via a
//! broadcast channel that IS Send — we subscribe to it on the
//! connection thread alongside the WebSocket receiver.

use crate::AppState;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use std::thread;
use tokio::sync::{broadcast, mpsc, RwLock};
use yrs::sync::protocol::{Message as YrsMessage, SyncMessage};
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Doc, ReadTxn, Transact, Update};

type DocBroadcast = broadcast::Sender<Vec<u8>>;

static DOC_ROOMS: LazyLock<RwLock<HashMap<String, DocBroadcast>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, Deserialize)]
pub struct WsParams {
    pub token: String,
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(doc_id): Path<String>,
    Query(_params): Query<WsParams>,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // Channels that bridge the Send boundary: the (Send) axum runtime
    // sends the upgraded socket to the (non-Send) connection thread.
    let (tx, mut rx) = mpsc::channel::<WebSocket>(1);
    let doc_id_clone = doc_id.clone();

    // Spawn a dedicated OS thread for this connection. The thread runs
    // its own single-threaded tokio runtime so yrs' Rc<RefCell<...>>
    // internals never cross a thread boundary.
    let _handle = thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build connection runtime");
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, async {
            let socket = rx.recv().await.expect("websocket channel closed");
            run_sync_session(socket, &doc_id_clone).await;
        });
    });

    ws.on_upgrade(move |socket| async move {
        let _ = tx.send(socket).await;
    })
}

async fn run_sync_session(socket: WebSocket, doc_id: &str) {
    let doc = Doc::new();
    let (mut sender, mut receiver) = socket.split();

    fn doc_channel() -> (broadcast::Sender<Vec<u8>>, broadcast::Receiver<Vec<u8>>) {
        broadcast::channel(1024)
    }

    let tx = {
        let read = DOC_ROOMS.read().await;
        if let Some(tx) = read.get(doc_id) {
            tx.clone()
        } else {
            let (tx, _) = doc_channel();
            let mut write = DOC_ROOMS.write().await;
            write
                .entry(doc_id.to_string())
                .or_insert_with(|| {
                    let (tx2, _) = doc_channel();
                    tx2
                })
                .clone()
        }
    };
    let mut rx = tx.subscribe();

    // Send sync-step-1 on connect.
    {
        let txn = doc.transact();
        let sv = txn.state_vector();
        let msg = YrsMessage::Sync(SyncMessage::SyncStep1(sv));
        let encoded = msg.encode_v1();
        if sender.send(Message::Binary(encoded)).await.is_err() {
            return;
        }
    }

    loop {
        tokio::select! {
            ws_msg = receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Binary(data))) => {
                        let mut decoder: yrs::updates::decoder::DecoderV1 = data.as_slice().into();
                        if let Ok(yrs_msg) = YrsMessage::decode(&mut decoder) {
                            match yrs_msg {
                                YrsMessage::Sync(SyncMessage::SyncStep1(_sv)) => {
                                    let txn = doc.transact();
                                    let sv = txn.state_vector();
                                    let reply = YrsMessage::Sync(SyncMessage::SyncStep2(
                                        txn.encode_state_as_update_v1(&sv),
                                    ));
                                    let bytes = reply.encode_v1();
                                    let _ = sender.send(Message::Binary(bytes)).await;
                                }
                                YrsMessage::Sync(SyncMessage::SyncStep2(update)) => {
                                    if let Ok(update) = Update::decode_v1(&update) {
                                        let mut txn = doc.transact_mut();
                                        let _ = txn.apply_update(update);
                                    }
                                }
                                YrsMessage::Sync(SyncMessage::Update(update)) => {
                                    if let Ok(update) = Update::decode_v1(&update) {
                                        let mut txn = doc.transact_mut();
                                        let _ = txn.apply_update(update);
                                    }
                                    let _ = tx.send(data);
                                }
                                _ => {}
                            }
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        let _ = sender.send(Message::Pong(p)).await;
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                }
            }
            broadcast_bytes = rx.recv() => {
                match broadcast_bytes {
                    Ok(bytes) => {
                        if sender.send(Message::Binary(bytes)).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    let snapshot = { let txn = doc.transact(); txn.encode_state_as_update_v1(&yrs::StateVector::default()) };
    let _ = snapshot;
}
