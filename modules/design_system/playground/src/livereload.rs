use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Extension;
use axum::response::IntoResponse;
use tokio::sync::broadcast;

// TODO: clean this all up,
//       ensure the conditional compilation is applied everywhere,
//       make it generalizeable

pub async fn reload_ws(ws: WebSocketUpgrade, Extension(notifier): Extension<ReloadNotifier>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, notifier))
}

async fn handle_ws(mut socket: WebSocket, notifier: ReloadNotifier) {
    let mut rx = notifier.subscribe();

    tokio::spawn(async move {
        while rx.recv().await.is_ok() {
            if socket.send(Message::Text("reload".into())).await.is_err() {
                break;
            }
        }
    });
}

pub async fn trigger_reload(Extension(notifier): Extension<ReloadNotifier>) {
    if cfg!(debug_assertions) {
        notifier.notify_reload();
    }
}

#[derive(Clone)]
pub struct ReloadNotifier {
    tx: broadcast::Sender<()>,
}

impl ReloadNotifier {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        ReloadNotifier { tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    pub fn notify_reload(&self) {
        let _ = self.tx.send(());
    }
}
