pub mod client_agent;
pub mod command;
pub mod db;
pub mod hub;
pub mod player;
pub mod proto;
pub mod proto_util;
pub mod spore;
pub mod util;

use nanoid::nanoid;
use std::{error::Error, net::SocketAddr};
use tokio::{net::TcpStream, sync::mpsc::UnboundedSender};
use tracing::info;

pub async fn handle_tcp_stream(
    tcp_stream: TcpStream,
    socket_addr: SocketAddr,
    db_pool: sqlx::Pool<sqlx::Sqlite>,
    hub_command_sender: UnboundedSender<command::Command>,
) -> Result<(), Box<dyn Error>> {
    let ws_stream = tokio_tungstenite::accept_async(tcp_stream).await?;
    info!("Accept WebSocket stream: {:?}", ws_stream);

    let connection_id = nanoid!();

    let client_agent_task = {
        let connection_id = connection_id.clone();
        let hub_command_sender = hub_command_sender.clone();
        let client_agent = client_agent::ClientAgent {
            socket_addr,
            connection_id,
            db_pool,
            hub_command_sender,
        };
        tokio::spawn(async move { client_agent.run(ws_stream).await })
    };

    let _ = client_agent_task.await;

    let unregister_command = command::Command::UnregisterClient { connection_id };
    let _ = hub_command_sender.send(unregister_command);

    Ok(())
}
