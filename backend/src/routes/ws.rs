use crate::{AppData, ops::models::LoggedInUser};
use axum::{
    extract::{
        ws::{CloseFrame, Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};

pub async fn ws_handler(
    State(app): State<AppData>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, app))
}

async fn handle_socket(mut socket: WebSocket, app: AppData) {
    let mut user_id = None;

    // Wait for auth
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Ping(data) => {
                let _ = socket.send(Message::Pong(data)).await;
            }
            Message::Text(ident) => {
                match LoggedInUser::authorize(&app.pool, &ident.as_str(), None).await {
                    Ok(Some(lu)) => {
                        user_id = Some(lu.user_id);
                        break;
                    }
                    Ok(None) => break,
                    Err(e) => {
                        let _ = socket
                            .send(Message::Close(Some(CloseFrame {
                                code: 4001,
                                reason: e.to_string().into(),
                            })))
                            .await;
                        return;
                    }
                };
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    let Some(user_id) = user_id else {
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
