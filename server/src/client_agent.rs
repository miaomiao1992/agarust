use crate::*;

use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use prost::Message as _;
use sqlx::query_as;
use std::{io::Cursor, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, RwLock,
    },
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{error, warn};

#[derive(Debug)]
pub struct ClientAgent {
    pub socket_addr: SocketAddr,
    pub connection_id: Arc<str>,
    pub db_pool: sqlx::Pool<sqlx::Sqlite>,
    pub hub_command_sender: UnboundedSender<command::Command>,
    pub client_agent_command_sender: UnboundedSender<command::Command>,
    pub db_player: Arc<RwLock<Option<db::Player>>>,
}

impl ClientAgent {
    pub fn new(
        socket_addr: SocketAddr,
        connection_id: Arc<str>,
        db_pool: sqlx::Pool<sqlx::Sqlite>,
        hub_command_sender: UnboundedSender<command::Command>,
    ) -> (Self, UnboundedReceiver<command::Command>) {
        let (client_agent_command_sender, client_agent_command_receiver) =
            unbounded_channel::<command::Command>();

        let db_player = Arc::new(RwLock::new(None));

        let client_agent = Self {
            socket_addr,
            connection_id,
            db_pool,
            hub_command_sender,
            client_agent_command_sender,
            db_player,
        };

        (client_agent, client_agent_command_receiver)
    }
}

pub async fn run(
    client_agent: Arc<ClientAgent>,
    client_agent_command_receiver: UnboundedReceiver<command::Command>,
    ws_stream: WebSocketStream<TcpStream>,
) {
    let (client_writer, client_reader) = ws_stream.split();

    let mut client_writer_task = {
        let client_agent = client_agent.clone();
        tokio::spawn(async move {
            client_writer_pump(client_agent, client_agent_command_receiver, client_writer).await
        })
    };

    let mut client_reader_task = {
        let client_agent = client_agent.clone();
        tokio::spawn(async move {
            let client_agent = client_agent.clone();
            client_reader_pump(client_agent, client_reader).await
        })
    };

    tokio::select! {
        _ = (&mut client_reader_task) => client_writer_task.abort(),
        _ = (&mut client_writer_task) => client_reader_task.abort(),
    };
}

async fn client_reader_pump(
    client_agent: Arc<ClientAgent>,
    mut client_reader: SplitStream<WebSocketStream<TcpStream>>,
) {
    while let Some(read_result) = client_reader.next().await {
        match read_result {
            Ok(message) => {
                if let Message::Binary(bytes) = message {
                    match proto::Packet::decode(Cursor::new(bytes)) {
                        Ok(packet) => {
                            let client_agent = client_agent.clone();
                            handle_client_reader_packet(client_agent, packet).await;
                        }
                        Err(error) => {
                            warn!(
                                "proto decode error {:?}: {:?}",
                                client_agent.socket_addr, error
                            );
                            continue;
                        }
                    }
                }
            }
            Err(error) => {
                warn!(
                    "client_reader error {:?}: {:?}",
                    client_agent.socket_addr, error
                );
                break;
            }
        }
    }
    warn!("exit client_reader_pump");
}

async fn client_writer_pump(
    client_agent: Arc<ClientAgent>,
    mut client_agent_command_receiver: UnboundedReceiver<command::Command>,
    client_writer: SplitSink<WebSocketStream<TcpStream>, Message>,
) {
    let client_writer = Arc::new(Mutex::new(client_writer));
    while let Some(command) = client_agent_command_receiver.recv().await {
        let client_writer = client_writer.clone();
        match command {
            command::Command::SendPacket { packet } => {
                let raw_data = packet.encode_to_vec();
                let mut client_writer = client_writer.lock().await;
                let _ = client_writer.send(Message::binary(raw_data)).await;
            }
            command::Command::SendRawData { raw_data } => {
                let mut client_writer = client_writer.lock().await;
                let _ = client_writer.send(Message::binary(raw_data)).await;
            }
            command::Command::UpdateSporeBatch { spore_batch } => {
                tokio::spawn(async move {
                    for spore_window in spore_batch.windows(32) {
                        let packet = proto_util::update_spore_batch_packet(spore_window);
                        let raw_data = packet.encode_to_vec();
                        {
                            let mut client_writer = client_writer.lock().await;
                            let _ = client_writer.send(Message::binary(raw_data)).await;
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                });
            }
            command::Command::SyncPlayerBestScore { current_score } => {
                let db_player_id = {
                    let mut db_player = client_agent.db_player.write().await;
                    let db_player = match &mut *db_player {
                        Some(db_player) => db_player,
                        None => {
                            warn!("sync player best score without login");
                            continue;
                        }
                    };
                    if db_player.best_score > current_score {
                        continue;
                    }

                    db_player.best_score = current_score;

                    db_player.id
                };

                let query_result = query_as!(
                    db::Player,
                    r#"UPDATE player SET best_score = ? WHERE id = ?"#,
                    current_score,
                    db_player_id,
                )
                .execute(&client_agent.db_pool)
                .await;

                if let Err(error) = query_result {
                    warn!("UPDATE player SET best_score error: {:?}", error);
                    continue;
                }
            }
            command::Command::DisconnectClinet => {
                warn!("Command::DisconnectClinet");
                let mut client_writer = client_writer.lock().await;
                let _ = client_writer.close().await;
                break;
            }
            _ => {
                warn!("unknown command: {:?}", command);
            }
        }
    }
    warn!("exit client_writer_pump");
}

async fn handle_client_reader_packet(client_agent: Arc<ClientAgent>, packet: proto::Packet) {
    if let Some(data) = packet.data {
        match data {
            proto::packet::Data::Login(login) => {
                let username = login.username;
                let password = login.password;

                let query_result = query_as!(
                    db::Auth,
                    r#"SELECT * FROM auth WHERE username = ? LIMIT 1"#,
                    username
                )
                .fetch_one(&client_agent.db_pool)
                .await;

                let auth = match query_result {
                    Ok(auth) => auth,
                    Err(error) => {
                        warn!("auth query error: {:?}", error);
                        let packet =
                            proto_util::login_err_packet("incorrect username or password".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                };

                match bcrypt::verify(password, &auth.password) {
                    Ok(valid) => {
                        if !valid {
                            warn!("bcrypt valid false");
                            let packet = proto_util::login_err_packet(
                                "incorrect username or password".into(),
                            );
                            let _ = client_agent
                                .client_agent_command_sender
                                .send(command::Command::SendPacket { packet });
                            return;
                        }
                    }
                    Err(error) => {
                        warn!("bcrypt verify error: {:?}", error);
                        let packet =
                            proto_util::login_err_packet("incorrect username or password".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                }

                let query_result = query_as!(
                    db::Player,
                    r#"SELECT * FROM player WHERE auth_id = ? LIMIT 1"#,
                    auth.id
                )
                .fetch_one(&client_agent.db_pool)
                .await;

                let player = match query_result {
                    Ok(player) => player,
                    Err(error) => {
                        warn!("player query error: {:?}", error);
                        let packet =
                            proto_util::login_err_packet("incorrect username or password".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                };

                {
                    let mut db_player = client_agent.db_player.write().await;
                    *db_player = Some(player);
                }

                let packet = proto_util::login_ok_packet();
                let _ = client_agent
                    .client_agent_command_sender
                    .send(command::Command::SendPacket { packet });
            }
            proto::packet::Data::Register(register) => {
                let username = register.username;
                let password = register.password;
                let color = register.color;

                let mut transaction = match client_agent.db_pool.begin().await {
                    Ok(transaction) => transaction,
                    Err(error) => {
                        warn!("transaction begin error: {:?}", error);
                        let packet =
                            proto_util::register_err_packet("transaction begin error".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                };

                if username.is_empty() {
                    warn!("username is empty: {:?}", username);
                    let packet = proto_util::register_err_packet("username is empty".into());
                    let _ = client_agent
                        .client_agent_command_sender
                        .send(command::Command::SendPacket { packet });
                    return;
                }

                if username.len() > 16 {
                    warn!("username too long: {:?}", username);
                    let packet = proto_util::register_err_packet("username too long".into());
                    let _ = client_agent
                        .client_agent_command_sender
                        .send(command::Command::SendPacket { packet });
                    return;
                }

                let query_result = query_as!(
                    db::Auth,
                    r#"SELECT * FROM auth WHERE username = ? LIMIT 1"#,
                    username
                )
                .fetch_one(&mut *transaction)
                .await;

                if query_result.is_ok() {
                    warn!("auth already exists: {:?}", username);
                    let packet = proto_util::register_err_packet("username already exists".into());
                    let _ = client_agent
                        .client_agent_command_sender
                        .send(command::Command::SendPacket { packet });
                    return;
                }

                let password = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
                    Ok(password) => password,
                    Err(error) => {
                        warn!("password hash error: {:?}", error);
                        let packet = proto_util::register_err_packet("password hash error".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                };

                let query_result = query_as!(
                    db::Auth,
                    r#"INSERT INTO auth ( username, password ) VALUES ( ?, ? )"#,
                    username,
                    password,
                )
                .execute(&mut *transaction)
                .await;

                let auth_id = match query_result {
                    Ok(query_result) => query_result.last_insert_rowid(),
                    Err(error) => {
                        warn!("auth insert error: {:?}", error);
                        let packet = proto_util::register_err_packet("auth insert error".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                };

                let query_result = query_as!(
                    db::Player,
                    r#"INSERT INTO player ( auth_id, nickname, color ) VALUES ( ?, ?, ? )"#,
                    auth_id,
                    username,
                    color,
                )
                .execute(&mut *transaction)
                .await;

                if let Err(error) = query_result {
                    warn!("player insert error: {:?}", error);
                    let packet = proto_util::register_err_packet("player insert error".into());
                    let _ = client_agent
                        .client_agent_command_sender
                        .send(command::Command::SendPacket { packet });
                    return;
                }

                if let Err(error) = transaction.commit().await {
                    warn!("transaction commit error: {:?}", error);
                    let packet = proto_util::register_err_packet("transaction commit error".into());
                    let _ = client_agent
                        .client_agent_command_sender
                        .send(command::Command::SendPacket { packet });
                    return;
                }

                let packet = proto_util::register_ok_packet();
                let _ = client_agent
                    .client_agent_command_sender
                    .send(command::Command::SendPacket { packet });
            }
            proto::packet::Data::Join(_) => {
                let db_player = client_agent.db_player.read().await;
                let db_player = match &*db_player {
                    Some(db_player) => db_player,
                    None => {
                        warn!("join without login");
                        let packet =
                            proto_util::register_err_packet("transaction commit error".into());
                        let _ = client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendPacket { packet });
                        return;
                    }
                };
                let _ = client_agent
                    .hub_command_sender
                    .send(command::Command::Join {
                        connection_id: client_agent.connection_id.clone(),
                        player_db_id: db_player.id,
                        nickname: db_player.nickname.clone(),
                        color: db_player.color,
                    });
            }
            proto::packet::Data::Chat(chat) => {
                let _ = client_agent
                    .hub_command_sender
                    .send(command::Command::Chat {
                        connection_id: client_agent.connection_id.clone(),
                        msg: chat.msg.into(),
                    });
            }
            proto::packet::Data::UpdatePlayerDirectionAngle(update_player_direction_angle) => {
                let _ = client_agent.hub_command_sender.send(
                    command::Command::UpdatePlayerDirectionAngle {
                        connection_id: client_agent.connection_id.clone(),
                        direction_angle: update_player_direction_angle.direction_angle,
                    },
                );
            }
            proto::packet::Data::ConsumeSpore(consume_spore) => {
                let _ = client_agent
                    .hub_command_sender
                    .send(command::Command::ConsumeSpore {
                        connection_id: client_agent.connection_id.clone(),
                        spore_id: consume_spore.spore_id.into(),
                    });
            }
            proto::packet::Data::ConsumePlayer(consume_player) => {
                let _ = client_agent
                    .hub_command_sender
                    .send(command::Command::ConsumePlayer {
                        connection_id: client_agent.connection_id.clone(),
                        victim_connection_id: consume_player.victim_connection_id.into(),
                    });
            }
            proto::packet::Data::Rush(_) => {
                let _ = client_agent
                    .hub_command_sender
                    .send(command::Command::Rush {
                        connectin_id: client_agent.connection_id.clone(),
                    });
            }
            proto::packet::Data::Disconnect(_) => {
                let _ = client_agent
                    .client_agent_command_sender
                    .send(command::Command::DisconnectClinet);
            }
            proto::packet::Data::LeaderboardRequest(_) => {
                let query_result = query_as!(
                    db::Player,
                    r#"SELECT * FROM player ORDER BY best_score DESC LIMIT ?"#,
                    100,
                )
                .fetch_all(&client_agent.db_pool)
                .await;

                let leaderboard_entry_list = match query_result {
                    Ok(player_list) => player_list
                        .iter()
                        .enumerate()
                        .map(|(index, player)| command::LeaderboardEntry {
                            rank: (index + 1) as u64,
                            player_nickname: player.nickname.clone(),
                            score: player.best_score as u64,
                        })
                        .collect::<Vec<_>>(),
                    Err(error) => {
                        error!("fetch leaderboard error: {:?}", error);
                        return;
                    }
                };

                let packet = proto_util::leaderboard_response(&leaderboard_entry_list);
                let _ = client_agent
                    .client_agent_command_sender
                    .send(command::Command::SendPacket { packet });
            }
            _ => {
                warn!("unknown packet data: {:?}", data);
            }
        }
    }
}
