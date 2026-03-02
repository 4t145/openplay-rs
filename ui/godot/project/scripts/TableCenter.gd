extends Control
class_name TableCenter

@onready var hole_cards_container = $HoleCardsContainer
@onready var action_left = $Actions/LeftAction
@onready var action_right = $Actions/RightAction
@onready var action_me = $Actions/MyAction
@onready var stage_label = $StageLabel

func clear_actions():
	action_left.text = ""
	action_right.text = ""
	action_me.text = ""

func set_hole_cards(cards_data: Array):
	for child in hole_cards_container.get_children():
		child.queue_free()
	
	var card_scene = load("res://scenes/Card.tscn")
	for card_info in cards_data:
		var card = card_scene.instantiate()
		card.suit = card_info.suit
		card.rank = card_info.rank
		hole_cards_container.add_child(card)

func set_action(seat: String, action_text: String):
	match seat:
		"left": action_left.text = action_text
		"right": action_right.text = action_text
		"me": action_me.text = action_text
