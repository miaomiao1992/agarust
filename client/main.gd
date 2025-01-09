extends Node2D

func _ready() -> void:
	WsClient.connect_to_url("ws://127.0.0.1:8080")
	WsClient.connected.connect(_on_ws_connected)
	WsClient.packet_received.connect(_on_ws_packet_received)

func _on_ws_connected() -> void:
	var packet := Global.proto.Packet.new()
	var chat := packet.new_chat()
	chat.set_msg("hello rust")
	WsClient.send(packet)

func _on_ws_packet_received(packet: Global.proto.Packet) -> void:
	print(packet)
