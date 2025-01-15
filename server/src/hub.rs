use crate::*;

use hashbrown::HashMap;
use prost::Message;
use std::time::Duration;
use tokio::{
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    time::{interval, Instant},
};
use tracing::{error, info, warn};

const TICK_DURATION: Duration = Duration::from_millis(50);
const SPAWN_SPORE_DURATION: Duration = Duration::from_millis(2000);
const MAX_SPORE_COUNT: usize = 1000;

#[derive(Debug, Clone)]
pub struct Client {
    pub client_agent: client_agent::ClientAgent,
    pub player: Option<player::Player>,
}

#[derive(Debug)]
pub struct Hub {
    pub client_map: HashMap<Arc<str>, Client>,
    pub spore_map: HashMap<String, spore::Spore>,
    pub command_sender: UnboundedSender<command::Command>,
    pub command_receiver: UnboundedReceiver<command::Command>,
    pub db_pool: sqlx::Pool<sqlx::Sqlite>,
}

impl Hub {
    pub fn new(db_pool: sqlx::Pool<sqlx::Sqlite>) -> Self {
        let (command_sender, command_receiver) = unbounded_channel::<command::Command>();
        Self {
            client_map: HashMap::new(),
            spore_map: HashMap::new(),
            command_sender,
            command_receiver,
            db_pool,
        }
    }

    pub async fn run(&mut self) {
        for _ in 0..MAX_SPORE_COUNT {
            self.spawn_spore();
        }

        let _ = self.command_sender.send(command::Command::Tick {
            last_tick: Instant::now(),
            interval: interval(TICK_DURATION),
        });

        let _ = self.command_sender.send(command::Command::SpawnSpore {
            interval: interval(SPAWN_SPORE_DURATION),
        });

        while let Some(command) = self.command_receiver.recv().await {
            self.handle_command(command).await;
        }
    }

    async fn handle_command(&mut self, command: command::Command) {
        match command {
            command::Command::RegisterClientAgent { client_agent } => {
                info!("RegisterClientAgent: {:?}", client_agent);

                let connection_id = client_agent.connection_id.clone();
                let client_agent_command_sender = client_agent.client_agent_command_sender.clone();

                {
                    let connection_id = connection_id.clone();
                    let client = Client {
                        client_agent,
                        player: None,
                    };
                    self.client_map.insert(connection_id, client);
                }

                let packet = proto_util::hello_packet(connection_id);
                let _ = client_agent_command_sender.send(command::Command::SendPacket { packet });
            }
            command::Command::UnregisterClientAgent { connection_id } => {
                info!("UnregisterClient: {:?}", connection_id);

                self.client_map.remove(&connection_id);

                let packet = proto_util::disconnect_packet(connection_id, "unregister".to_string());
                let _ = self
                    .command_sender
                    .send(command::Command::BroadcastPacket { packet });
            }
            command::Command::Join {
                connection_id,
                player_db_id,
                nickname,
                color,
            } => {
                info!(
                    "PlayerJoin: {:?} {:?} {:?} {:?}",
                    connection_id, player_db_id, nickname, color
                );

                self.client_map
                    .values()
                    .flat_map(|client| client.player.as_ref())
                    .for_each(|online_player| {
                        if online_player.db_id == player_db_id {
                            if let Some(online_client) =
                                self.client_map.get(&online_player.connection_id)
                            {
                                let _ = online_client
                                    .client_agent
                                    .client_agent_command_sender
                                    .send(command::Command::DisconnectClinet);
                            }
                        }
                    });

                let client = match self.client_map.get_mut(&connection_id) {
                    Some(client) => client,
                    None => {
                        error!("client not found: {:?}", connection_id);
                        return;
                    }
                };

                let player =
                    player::Player::random(player_db_id, connection_id.clone(), nickname, color);

                let player_x = player.x;
                let player_y = player.y;

                client.player = Some(player);

                let mut spore_batch = self.spore_map.values().cloned().collect::<Vec<_>>();
                spore_batch.sort_by_cached_key(|spore| {
                    ((player_x - spore.x).powi(2) + (player_y - spore.y).powi(2)) as i64
                });

                let _ = client
                    .client_agent
                    .client_agent_command_sender
                    .send(command::Command::UpdateSporeBatch { spore_batch });
            }
            command::Command::BroadcastPacket { packet } => {
                let raw_data = packet.encode_to_vec();
                let _ = self
                    .command_sender
                    .send(command::Command::BroadcastRawData { raw_data });
            }
            command::Command::BroadcastRawData { raw_data } => {
                self.client_map
                    .values()
                    .filter(|client| client.player.is_some())
                    .for_each(|client| {
                        let raw_data = raw_data.clone();
                        let _ = client
                            .client_agent
                            .client_agent_command_sender
                            .send(command::Command::SendRawData { raw_data });
                    });
            }
            command::Command::Tick {
                mut last_tick,
                mut interval,
            } => {
                let delta = last_tick.elapsed();

                self.tick_player(delta);
                last_tick = Instant::now();

                let _ = self.command_sender.send(command::Command::SyncPlayer);

                let hub_command_sender = self.command_sender.clone();
                tokio::spawn(async move {
                    interval.tick().await;
                    if let Err(error) = hub_command_sender.send(command::Command::Tick {
                        last_tick,
                        interval,
                    }) {
                        error!("send command::Command::Tick error: {:?}", error);
                    }
                });
            }
            command::Command::SyncPlayer => {
                let player_list = self
                    .client_map
                    .values()
                    .filter_map(|client| client.player.as_ref())
                    .collect::<Vec<_>>();

                let packet = proto_util::update_player_batch_packet(&player_list);
                let _ = self
                    .command_sender
                    .send(command::Command::BroadcastPacket { packet });
            }
            command::Command::Chat { connection_id, msg } => {
                let packet = proto_util::chat_packet(connection_id, msg);
                let _ = self
                    .command_sender
                    .send(command::Command::BroadcastPacket { packet });
            }
            command::Command::UpdatePlayerDirectionAngle {
                connection_id,
                direction_angle,
            } => {
                if let Some(client) = self.client_map.get_mut(&connection_id) {
                    if let Some(player) = client.player.as_mut() {
                        player.direction_angle = direction_angle;
                    }
                }
            }
            command::Command::ConsumeSpore {
                connection_id,
                spore_id,
            } => {
                if let (Some(client), Some(spore)) = (
                    self.client_map.get_mut(&connection_id),
                    self.spore_map.get_mut(&spore_id),
                ) {
                    if let Some(player) = client.player.as_mut() {
                        let is_close = util::check_distance_is_close(
                            player.x,
                            player.y,
                            player.radius,
                            spore.x,
                            spore.y,
                            spore.radius,
                        );

                        if !is_close {
                            warn!("consume spore error, distance too far");
                            return;
                        }

                        let spore_mass = util::radius_to_mass(spore.radius);
                        player.increase_mass(spore_mass);

                        self.spore_map.remove(&spore_id);

                        let packet = proto_util::consume_spore_packet(connection_id, spore_id);
                        let _ = self
                            .command_sender
                            .send(command::Command::BroadcastPacket { packet });

                        let current_score = util::radius_to_mass(player.radius) as i64;
                        let _ = client
                            .client_agent
                            .client_agent_command_sender
                            .send(command::Command::SyncPlayerBestScore { current_score });
                    }
                }
            }
            command::Command::ConsumePlayer {
                connection_id,
                victim_connection_id,
            } => {
                if let [Some(player_client), Some(victim_client)] = self
                    .client_map
                    .get_many_mut([&connection_id, &victim_connection_id])
                {
                    if let (Some(player), Some(victim)) =
                        (&mut player_client.player, &mut victim_client.player)
                    {
                        let is_close = util::check_distance_is_close(
                            player.x,
                            player.y,
                            player.radius,
                            victim.x,
                            victim.y,
                            victim.radius,
                        );

                        if !is_close {
                            warn!("consume player error, distance too far");
                            return;
                        }

                        let victim_mass = util::radius_to_mass(victim.radius);
                        player.increase_mass(victim_mass);

                        victim.respawn();
                    }
                }
            }
            command::Command::SpawnSpore { mut interval } => {
                if self.spore_map.len() < MAX_SPORE_COUNT {
                    let spore = spore::Spore::random();
                    self.spore_map.insert(spore.id.clone(), spore);
                }

                let hub_command_sender = self.command_sender.clone();
                tokio::spawn(async move {
                    interval.tick().await;
                    let _ = hub_command_sender.send(command::Command::SpawnSpore { interval });
                });
            }
            _ => {
                warn!("unknown command: {:?}", command);
            }
        }
    }

    fn spawn_spore(&mut self) {
        let spore = spore::Spore::random();
        self.spore_map.insert(spore.id.clone(), spore);
    }

    fn tick_player(&mut self, delta: Duration) {
        let delta_secs = delta.as_secs_f64();
        self.client_map
            .values_mut()
            .flat_map(|client| client.player.as_mut())
            .for_each(|player| {
                let new_x = player.x + player.speed * player.direction_angle.cos() * delta_secs;
                let new_y = player.y + player.speed * player.direction_angle.sin() * delta_secs;

                player.x = new_x;
                player.y = new_y;

                let drop_mass_probability = player.radius / (MAX_SPORE_COUNT as f64 * 2.0);
                if rand::random::<f64>() < drop_mass_probability {
                    if let Some(mass) = player.try_drop_mass() {
                        let mut spore = spore::Spore::random();
                        spore.x = player.x;
                        spore.y = player.y;
                        spore.radius = util::mass_to_radius(mass);

                        let packet = proto_util::update_spore_pack(&spore);
                        let _ = self
                            .command_sender
                            .send(command::Command::BroadcastPacket { packet });

                        self.spore_map.insert(spore.id.clone(), spore);
                    }
                }
            });
    }
}
