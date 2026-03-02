extends Control
class_name PlayerSeat

@onready var name_label = $VBoxContainer/NameLabel
@onready var status_label = $VBoxContainer/StatusLabel
@onready var hand_container = $HandContainer

@export var is_my_seat: bool = false

func update_info(player_name: String, is_ready: bool, role: String, card_count: int):
	name_label.text = player_name + (" (Me)" if is_my_seat else "")
	status_label.text = role + (" - Ready" if is_ready else " - Not Ready")
	if card_count > 0:
		status_label.text += " | Cards: " + str(card_count)

func clear_hand():
	for child in hand_container.get_children():
		child.queue_free()

func set_hand(cards_data: Array):
	clear_hand()
	var card_scene = load("res://scenes/Card.tscn")
	for card_info in cards_data:
		var card = card_scene.instantiate()
		card.suit = card_info.suit
		card.rank = card_info.rank
		hand_container.add_child(card)

func get_selected_cards() -> Array:
	var selected = []
	for i in range(hand_container.get_child_count()):
		var card = hand_container.get_child(i)
		if card.is_selected:
			selected.append({"suit": card.suit, "rank": card.rank, "index": i})
	return selected
