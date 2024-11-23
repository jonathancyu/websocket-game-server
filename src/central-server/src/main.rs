use server::service::{matchmaking::MatchmakingService, websocket::WebSocketHandler};
use server::utility::channel::Channel;
use tokio::{sync::mpsc, task::JoinHandle};

#[tokio::main]
async fn main() {
    let subscriber = tracing_subscriber::FmtSubscriber::new();
    let _ = tracing::subscriber::set_global_default(subscriber);
    let to_mm_channel = Channel::from(mpsc::channel(100));
    let to_ws_channel = Channel::from(mpsc::channel(100));

    // Spawn thread for matchmaking
    let matchmaker_handle: JoinHandle<()> = tokio::spawn(async move {
        MatchmakingService::new()
            .listen(to_ws_channel.sender, to_mm_channel.receiver)
            .await;
    });
    let websocket_handle: JoinHandle<()> = tokio::spawn(async move {
        WebSocketHandler::new("0.0.0.0".to_owned(), "3001".to_owned())
            .listen(to_mm_channel.sender, to_ws_channel.receiver)
            .await;
    });

    websocket_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
    matchmaker_handle
        .await
        .expect("Matchmaking thread exited non-gracefully");
}
