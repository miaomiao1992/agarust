// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Packet {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(oneof = "packet::Data", tags = "2, 3, 8, 9")]
    pub data: ::core::option::Option<packet::Data>,
}
/// Nested message and enum types in `Packet`.
pub mod packet {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Data {
        #[prost(message, tag = "2")]
        Hello(super::Hello),
        #[prost(message, tag = "3")]
        Chat(super::Chat),
        #[prost(message, tag = "8")]
        UpdatePlayer(super::UpdatePlayer),
        #[prost(message, tag = "9")]
        UpdatePlayerDirection(super::UpdatePlayerDirection),
    }
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Hello {}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Chat {
    #[prost(string, tag = "1")]
    pub msg: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdatePlayer {
    #[prost(string, tag = "1")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub name: ::prost::alloc::string::String,
    #[prost(double, tag = "3")]
    pub x: f64,
    #[prost(double, tag = "4")]
    pub y: f64,
    #[prost(double, tag = "5")]
    pub radius: f64,
    #[prost(double, tag = "6")]
    pub direction: f64,
    #[prost(double, tag = "7")]
    pub speed: f64,
    #[prost(int32, tag = "8")]
    pub color: i32,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct UpdatePlayerDirection {
    #[prost(double, tag = "1")]
    pub direction: f64,
}
