use std::net::SocketAddr;

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    http::Response,
    response::{Html, IntoResponse},
    routing::get,
    Router, Server,
};
use clap::Parser;
use serde::Serialize;
use sysinfo::{CpuExt, System, SystemExt};
use tokio::sync::broadcast;
use tower_http::compression::CompressionLayer;

macro_rules! read_asset(
    ($path:literal) => {
      if cfg!(feature = "dev") {
        tokio::fs::read(concat!("assets/", $path)).await.unwrap().to_owned().into()
      } else {
        std::borrow::Cow::<'static, [u8]>::from(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)).as_slice())
      }
    };
);

#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    Snapshot { cpus: Vec<f32> },
}

#[derive(Clone)]
struct AppState {
    tx: broadcast::Sender<Event>,
}

#[derive(Parser, Debug)]
struct Opts {
    #[arg(long, short, default_value = "[::]:0")]
    listen: SocketAddr,
}
#[tokio::main]
async fn main() {
    let opts = Opts::parse();

    let (tx, _) = broadcast::channel::<Event>(1);

    tracing_subscriber::fmt::init();

    let app_state = AppState { tx: tx.clone() };

    let router = Router::new()
        .route("/", get(root_get))
        .route("/favicon.ico", get(favicon_get))
        .route("/index.mjs", get(indexmjs_get))
        .route("/index.css", get(indexcss_get))
        .route("/events", get(events_get))
        .with_state(app_state.clone())
        .layer(CompressionLayer::new());

    // Update CPU usage in the background
    tokio::task::spawn_blocking(move || {
        let mut sys = System::new();
        loop {
            sys.refresh_cpu();
            let v: Vec<_> = sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect();
            let _ = tx.send(Event::Snapshot { cpus: v });
            std::thread::sleep(System::MINIMUM_CPU_UPDATE_INTERVAL);
        }
    });

    let server = Server::bind(&opts.listen).serve(router.into_make_service());
    let addr = server.local_addr();
    eprintln!("Listening on http://{addr}");

    server.await.unwrap();
}

#[axum::debug_handler]
async fn root_get() -> impl IntoResponse {
    let content = read_asset!("index.html");

    Html(content)
}

#[axum::debug_handler]
async fn indexmjs_get() -> impl IntoResponse {
    let content = read_asset!("index.mjs");

    Response::builder()
        .header("content-type", "application/javascript;charset=utf-8")
        .body(Body::from(content))
        .unwrap()
}

#[axum::debug_handler]
async fn favicon_get() -> impl IntoResponse {
    let content = read_asset!("favicon.ico");

    Response::builder()
        .header("content-type", "application/javascript;charset=utf-8")
        .body(Body::from(content))
        .unwrap()
}

#[axum::debug_handler]
async fn indexcss_get() -> impl IntoResponse {
    let content = read_asset!("index.css");

    Response::builder()
        .header("content-type", "text/css;charset=utf-8")
        .body(Body::from(content))
        .unwrap()
}

#[axum::debug_handler]
async fn events_get(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|ws: WebSocket| async { realtime_cpus_stream(state, ws).await })
}

async fn realtime_cpus_stream(app_state: AppState, mut ws: WebSocket) {
    let mut rx = app_state.tx.subscribe();

    while let Ok(msg) = rx.recv().await {
        ws.send(Message::Text(serde_json::to_string(&msg).unwrap()))
            .await
            .unwrap();
    }
}
