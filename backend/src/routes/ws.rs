use crate::AppData;
use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use sqlx::Row;
use uuid::Uuid;

pub async fn ws_handler(
    State(app): State<AppData>,
    jar: CookieJar,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user = authenticate(&app, &jar).await;
    ws.on_upgrade(move |socket| handle_socket(socket, app, user))
}

async fn authenticate(app: &AppData, jar: &CookieJar) -> Option<Uuid> {
    let token = jar.get("session_token")?.value().to_owned();
    let row = sqlx::query(
        "SELECT u.id FROM sessions s JOIN users u ON s.user_id = u.id WHERE s.token = $1 AND s.expires_at > NOW()",
    )
    .bind(&token)
    .fetch_optional(&app.pool)
    .await
    .ok()??;

    Some(row.get("id"))
}

async fn handle_socket(mut socket: WebSocket, app: AppData, user: Option<Uuid>) {
    let Some(user_id) = user else {
        let _ = socket
            .send(Message::Close(Some(CloseFrame {
                code: 4001,
                reason: "Unauthorized".into(),
            })))
            .await;
        return;
    };

    let mut rx = app.notify.subscribe(user_id);

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        let json = serde_json::to_string(&event).unwrap();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        let msg = serde_json::json!({"event": "Lagged", "missed": n});
                        let _ = socket.send(Message::Text(msg.to_string().into())).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    app.notify.remove_if_empty(user_id);
}
