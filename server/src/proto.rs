// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Packet {
    #[prost(
        oneof = "packet::Data",
        tags = "1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19"
    )]
    pub data: ::core::option::Option<packet::Data>,
}
/// Nested message and enum types in `Packet`.
pub mod packet {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "1")]
        Hello(super::Hello),
        #[prost(message, tag = "2")]
        Login(super::Login),
        #[prost(message, tag = "3")]
        LoginOk(super::LoginOk),
        #[prost(message, tag = "4")]
        LoginErr(super::LoginErr),
        #[prost(message, tag = "5")]
        Register(super::Register),
        #[prost(message, tag = "6")]
        RegisterOk(super::RegisterOk),
        #[prost(message, tag = "7")]
        RegisterErr(super::RegisterErr),
        #[prost(message, tag = "8")]
        Join(super::Join),
        #[prost(message, tag = "9")]
        Disconnect(super::Disconnect),
        #[prost(message, tag = "10")]
        Chat(super::Chat),
        #[prost(message, tag = "11")]
        UpdatePlayer(super::UpdatePlayer),
        #[prost(message, tag = "12")]
        UpdatePlayerBatch(super::UpdatePlayerBatch),
        #[prost(message, tag = "13")]
        UpdatePlayerDirectionAngle(super::UpdatePlayerDirectionAngle),
        #[prost(message, tag = "14")]
        UpdateSpore(super::UpdateSpore),
        #[prost(message, tag = "15")]
        UpdateSporeBatch(super::UpdateSporeBatch),
        #[prost(message, tag = "16")]
        ConsumeSpore(super::ConsumeSpore),
        #[prost(message, tag = "17")]
        ConsumePlayer(super::ConsumePlayer),
        #[prost(message, tag = "18")]
        LeaderboardRequest(super::LeaderboardRequest),
        #[prost(message, tag = "19")]
        LeaderboardResponse(super::LeaderboardResponse),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Hello {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Login {
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct LoginOk {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LoginErr {
    #[prost(string, tag = "1")]
    pub reason: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Register {
    #[prost(string, tag = "1")]
    pub username: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub password: ::prost::alloc::string::String,
    #[prost(int64, tag = "3")]
    pub color: i64,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct RegisterOk {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegisterErr {
    #[prost(string, tag = "1")]
    pub reason: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Join {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Disconnect {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub reason: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Chat {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub msg: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdatePlayer {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub nickname: ::prost::alloc::string::String,
    #[prost(double, tag = "3")]
    pub x: f64,
    #[prost(double, tag = "4")]
    pub y: f64,
    #[prost(double, tag = "5")]
    pub radius: f64,
    #[prost(double, tag = "6")]
    pub direction_angle: f64,
    #[prost(double, tag = "7")]
    pub speed: f64,
    #[prost(int64, tag = "8")]
    pub color: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdatePlayerBatch {
    #[prost(message, repeated, tag = "1")]
    pub update_player_batch: ::prost::alloc::vec::Vec<UpdatePlayer>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct UpdatePlayerDirectionAngle {
    #[prost(double, tag = "1")]
    pub direction_angle: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateSpore {
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    #[prost(double, tag = "2")]
    pub x: f64,
    #[prost(double, tag = "3")]
    pub y: f64,
    #[prost(double, tag = "4")]
    pub radius: f64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateSporeBatch {
    #[prost(message, repeated, tag = "1")]
    pub update_spore_batch: ::prost::alloc::vec::Vec<UpdateSpore>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConsumeSpore {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub spore_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConsumePlayer {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub victim_connection_id: ::prost::alloc::string::String,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct LeaderboardRequest {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LeaderboardEntry {
    #[prost(uint64, tag = "1")]
    pub rank: u64,
    #[prost(string, tag = "2")]
    pub player_nickname: ::prost::alloc::string::String,
    #[prost(uint64, tag = "3")]
    pub score: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct LeaderboardResponse {
    #[prost(message, repeated, tag = "1")]
    pub leaderboard_entry_list: ::prost::alloc::vec::Vec<LeaderboardEntry>,
}
