extends Node2D

@onready var world: Node2D = %World
@onready var logout_button: Button = %LogoutButton
@onready var chat_edit: LineEdit = %ChatEdit
@onready var send_chat_button: Button = %SendChatButton
@onready var leaderboard: Leaderboard = %Leaderboard
@onready var logger: Logger = %Logger

var player_map: Dictionary = {}
var spore_map: Dictionary = {}

func _ready() -> void:
	WsClient.connect_to_server(Global.server_url)
	WsClient.connected.connect(_on_ws_connected)
	WsClient.packet_received.connect(_on_ws_packet_received)
	chat_edit.text_submitted.connect(_on_chat_edit_text_submited)

func _on_ws_connected() -> void:
	logger.info("Server connected")

func _on_ws_packet_received(packet: Global.proto.Packet) -> void:
	if packet.has_hello():
		logger.info(packet.to_string())
		_handle_hello_msg(packet.get_hello())
	elif packet.has_chat():
		print_debug(packet)
		_handle_chat_msg(packet.get_chat())
	elif packet.has_update_player():
		print_debug(packet)
		_handle_update_player_msg(packet.get_update_player())
	elif packet.has_update_player_batch():
		_handle_update_player_batch_msg(packet.get_update_player_batch())
	elif packet.has_update_spore():
		_handle_update_spore_msg(packet.get_update_spore())
	elif packet.has_update_spore_batch():
		_handle_update_spore_batch_msg(packet.get_update_spore_batch())
	elif packet.has_consume_spore():
		_handle_consume_spore_msg(packet.get_consume_spore())
	elif packet.has_disconnect():
		_handle_disconnect_msg(packet.get_disconnect())
	else:
		print_debug("unknow packet: ", packet)

func _on_chat_edit_text_submited(new_text: String):
	var packet := Global.proto.Packet.new()
	var chat := packet.new_chat()
	chat.set_msg(new_text)
	WsClient.send(packet)
	chat_edit.text = ""

func _handle_hello_msg(hello_msg: Global.proto.Hello) -> void:
	Global.connection_id = hello_msg.get_connection_id()

func _handle_chat_msg(chat_msg: Global.proto.Chat) -> void:
	logger.chat(chat_msg.get_connection_id(), chat_msg.get_msg())

func _handle_update_player_batch_msg(update_player_batch_msg: Global.proto.UpdatePlayerBatch) -> void:
	for update_player_msg: Global.proto.UpdatePlayer in update_player_batch_msg.get_update_player_batch():
		_handle_update_player_msg(update_player_msg)

func _handle_update_player_msg(update_player_msg: Global.proto.UpdatePlayer) -> void:
	var actor_connection_id := update_player_msg.get_connection_id()
	var actor_name := update_player_msg.get_name()
	var x := update_player_msg.get_x()
	var y := update_player_msg.get_y()
	var radius := update_player_msg.get_radius()
	var speed := update_player_msg.get_speed()
	var color_hex := update_player_msg.get_color()

	var color := Color.hex(color_hex)
	var is_player := actor_connection_id == Global.connection_id

	if actor_connection_id not in player_map:
		_add_actor(actor_connection_id, actor_name, x, y, radius, speed, color, is_player)
	else:
		var direction := update_player_msg.get_direction_angle()
		_update_actor(actor_connection_id, x, y, direction, speed, radius, is_player)

func _handle_update_spore_batch_msg(update_spore_batch_msg: Global.proto.UpdateSporeBatch) -> void:
	for update_spore_msg: Global.proto.UpdateSpore in update_spore_batch_msg.get_update_spore_batch():
		_handle_update_spore_msg(update_spore_msg)

func _handle_update_spore_msg(update_spore_msg: Global.proto.UpdateSpore) -> void:
	var spore_id := update_spore_msg.get_id()
	var x := update_spore_msg.get_x()
	var y := update_spore_msg.get_y()
	var radius := update_spore_msg.get_radius()
	var underneath_player := false

	if Global.connection_id in player_map:
		var player = player_map[Global.connection_id]
		var player_pos := Vector2(player.position.x, player.position.y)
		var spore_pos := Vector2(x, y)
		underneath_player = player_pos.distance_squared_to(spore_pos) < player.radius * player.radius

	if spore_id not in spore_map:
		var spore := Spore.instantiate(spore_id, x, y, radius, underneath_player)
		world.add_child(spore)
		spore_map[spore_id] = spore

func _handle_consume_spore_msg(consume_spore_msg: Global.proto.ConsumeSpore) -> void:
	var connection_id := consume_spore_msg.get_connection_id()
	var spore_id := consume_spore_msg.get_spore_id()
	if connection_id in player_map and spore_id in spore_map:
		var actor = player_map[connection_id]
		var actor_mass := _radius_to_mass(actor.radius)

		var spore = spore_map[spore_id]
		var spore_mass := _radius_to_mass(spore.radius)

		_set_actor_mass(actor, actor_mass + spore_mass)
		_remove_spore(spore)

func _handle_disconnect_msg(disconnect_msg: Global.proto.Disconnect) -> void:
	var connection_id = disconnect_msg.get_connection_id()
	if connection_id in player_map:
		var player = player_map[connection_id]
		var reason := disconnect_msg.get_reason()
		logger.info("%s disconnected because %s" % [player.actor_name, reason])
		_remove_actor(player)

func _add_actor(connection_id: String, actor_name: String, x: float, y: float, radius: float, speed: float, color: Color, is_player: bool) -> void:
	var actor := Actor.instantiate(connection_id, actor_name, x, y, radius, speed, color, is_player)
	actor.z_index = 1
	world.add_child(actor)
	var mass := _radius_to_mass(radius)
	_set_actor_mass(actor, mass)
	player_map[connection_id] = actor

	if is_player:
		actor.area_entered.connect(_on_player_area_entered)

func _radius_to_mass(radius: float) -> float:
	return radius * radius * PI

func _set_actor_mass(actor: Actor, mass: float) -> void:
	actor.radius = sqrt(mass / PI)
	leaderboard.set_score(actor.actor_name, roundi(mass))

func _on_player_area_entered(area: Area2D) -> void:
	if area is Spore:
		_consume_spore(area as Spore)
	elif area is Actor:
		_collide_actor(area as Actor)

func _consume_spore(spore: Spore) -> void:
	if spore.underneath_player:
		return

	var player = player_map[Global.connection_id]
	var player_mass := _radius_to_mass(player.radius)
	var spore_mass := _radius_to_mass(spore.radius)
	_set_actor_mass(player, player_mass + spore_mass)

	var packet := Global.proto.Packet.new()
	var consume_spore_msg := packet.new_consume_spore()
	consume_spore_msg.set_spore_id(spore.spore_id)
	WsClient.send(packet)
	_remove_spore(spore)

func _remove_spore(spore: Spore) -> void:
	spore_map.erase(spore.spore_id)
	spore.queue_free()

func _collide_actor(actor: Actor) -> void:
	var player = player_map[Global.connection_id]
	var player_mass := _radius_to_mass(player.radius)
	var actor_mass := _radius_to_mass(actor.radius)

	if player_mass > actor_mass * 1.5:
		_consume_actor(actor)

func _consume_actor(actor: Actor) -> void:
	var player = player_map[Global.connection_id]
	var player_mass := _radius_to_mass(player.radius)
	var actor_mass := _radius_to_mass(actor.radius)
	_set_actor_mass(player, player_mass + actor_mass)

	var packet := Global.proto.Packet.new()
	var consume_player_msg := packet.new_consume_player()
	consume_player_msg.set_victim_connection_id(actor.connection_id)
	WsClient.send(packet)
	_remove_actor(actor)

func _remove_actor(actor: Actor) -> void:
	player_map.erase(actor.connection_id)
	actor.queue_free()
	leaderboard.remove(actor.actor_name)

func _update_actor(connection_id: String, x: float, y: float, direction: float, speed: float, radius: float, is_player: bool) -> void:
	var actor = player_map[connection_id]

	_set_actor_mass(actor, _radius_to_mass(radius))

	actor.speed = speed

	var server_position := Vector2(x, y)
	if actor.position.distance_squared_to(server_position) > 50:
		actor.server_position = server_position

	if not is_player:
		actor.direction = Vector2.from_angle(direction)
