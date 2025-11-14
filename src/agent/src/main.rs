use agent::{client::Client, strategy::*};
use common::{
    message::game_server::{ClientRequest, ClientResponse},
    model::messages::{Id, OpenSocketRequest},
};
use futures_util::{SinkExt, StreamExt};
use serde_json;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_line_number(true)
        .with_file(true)
        .init();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let strategy_name = args.get(1).map(|s| s.as_str()).unwrap_or("Random");
    let matchmaking_url = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("ws://localhost:3001");
    let player_id = args
        .get(3)
        .map(|s| {
            let uuid = Uuid::parse_str(s).expect("Invalid UUID format");
            Id(uuid)
        })
        .unwrap_or_else(|| Id::new());

    info!(
        "Starting agent with strategy: {}, player_id: {:?}, matchmaking_url: {}",
        strategy_name, player_id, matchmaking_url
    );

    // Create strategy
    let strategy: Box<dyn Strategy> = match strategy_name {
        "OnlyRock" => Box::new(OnlyRock {}),
        "OnlyPaper" => Box::new(OnlyPaper {}),
        "OnlyScissors" => Box::new(OnlyScissors {}),
        "Random" => Box::new(RandomMove {}),
        "CopyLastMove" => Box::new(CopyLastMove {}),
        "BeatLastMove" => Box::new(BeatLastMove {}),
        "FrequencyCounter" => Box::new(FrequencyCounter {}),
        "Rotate" => Box::new(Rotate {}),
        _ => {
            error!("Unknown strategy: {}. Using Random.", strategy_name);
            Box::new(RandomMove {})
        }
    };

    let mut client = Client::new(strategy);

    // Connect to matchmaking server
    info!("Connecting to matchmaking server: {}", matchmaking_url);
    let (ws_stream, _) = connect_async(matchmaking_url)
        .await
        .expect("Failed to connect to matchmaking server");

    let (mut write, mut read) = ws_stream.split();

    // Send user identification
    let open_request = OpenSocketRequest {
        user_id: player_id,
    };
    write
        .send(Message::Text(serde_json::to_string(&open_request).unwrap()))
        .await
        .expect("Failed to send identification");

    // Send join queue request
    let join_queue = serde_json::json!({"type": "JoinQueue"});
    write
        .send(Message::Text(join_queue.to_string()))
        .await
        .expect("Failed to send join queue");

    info!("Joined matchmaking queue");

    // Wait for match found
    let mut game_server_address: Option<String> = None;
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response: serde_json::Value =
                    serde_json::from_str(&text).expect("Failed to parse matchmaking response");

                if let Some(msg_type) = response.get("type").and_then(|t| t.as_str()) {
                    match msg_type {
                        "JoinedQueue" => info!("Successfully joined queue"),
                        "QueuePing" => {
                            // Keep connection alive
                        }
                        "MatchFound" => {
                            if let Some(server_address) = response.get("server_address") {
                                if let Some(addr) = server_address.as_str() {
                                    game_server_address = Some(addr.to_string());
                                    info!("Match found! Connecting to game server: {}", addr);
                                    break;
                                }
                            }
                        }
                        _ => info!("Received matchmaking message: {}", text),
                    }
                }
            }
            Ok(Message::Close(_)) => {
                warn!("Matchmaking connection closed");
                return;
            }
            Err(e) => {
                error!("Error reading from matchmaking: {}", e);
                return;
            }
            _ => {}
        }
    }

    let game_server_address = game_server_address.expect("Did not receive game server address");

    // Connect to game server
    let game_url = format!("ws://{}", game_server_address);
    info!("Connecting to game server: {}", game_url);
    let (ws_stream, _) = connect_async(&game_url)
        .await
        .expect("Failed to connect to game server");

    let (mut write, mut read) = ws_stream.split();

    // Send user identification
    write
        .send(Message::Text(serde_json::to_string(&open_request).unwrap()))
        .await
        .expect("Failed to send identification");

    // Send join game request
    let join_game = serde_json::json!({"type": "JoinGame"});
    write
        .send(Message::Text(join_game.to_string()))
        .await
        .expect("Failed to send join game");

    info!("Joined game. Strategy: {}", client.strategy_name());

    // Game loop
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let response: ClientResponse =
                    serde_json::from_str(&text).expect("Failed to parse game response");

                match response {
                    ClientResponse::GameJoined => {
                        info!("Game joined successfully");
                    }
                    ClientResponse::PendingMove => {
                        // Make a move
                        let move_to_play = client.play();
                        info!("Playing move: {:?}", move_to_play);
                        let move_request = ClientRequest::Move {
                            value: move_to_play,
                        };
                        write
                            .send(Message::Text(serde_json::to_string(&move_request).unwrap()))
                            .await
                            .expect("Failed to send move");
                    }
                    ClientResponse::RoundResult(result) => {
                        info!(
                            "Round result: {:?}, opponent played: {:?}",
                            result.result, result.other_move
                        );
                        client.record_round(result.other_move, result.result);
                    }
                    ClientResponse::MatchResult { result, wins, total } => {
                        info!(
                            "Match finished! Result: {:?}, Wins: {}/{}, Strategy: {}",
                            result, wins, total, client.strategy_name()
                        );
                        break;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Game connection closed");
                break;
            }
            Err(e) => {
                error!("Error reading from game server: {}", e);
                break;
            }
            _ => {}
        }
    }

    info!("Agent finished");
}
