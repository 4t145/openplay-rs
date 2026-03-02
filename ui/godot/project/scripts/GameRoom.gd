extends Control
class_name GameRoom

@onready var seat_me = $Seats/MySeat
@onready var seat_left = $Seats/LeftSeat
@onready var seat_right = $Seats/RightSeat
@onready var table_center = $TableCenter
@onready var ui_layer = $UI
@onready var ready_btn = $UI/ReadyBtn
@onready var start_btn = $UI/StartBtn
@onready var bid_container = $UI/BidContainer
@onready var play_container = $UI/PlayContainer

# State tracking (mocked for demo)
var game_stage = "Waiting" # Waiting, Bidding, Playing, Finished

func _ready():
	# Initially in Waiting state
	_update_ui_state("Waiting")
	
	# Connect buttons (in real code, these would send signals to Server)
	ready_btn.pressed.connect(_on_ready_pressed)
	start_btn.pressed.connect(_on_start_pressed)
	
	# Connect Bid buttons
	bid_container.get_node("PassBtn").pressed.connect(_on_bid_pressed.bind(0))
	bid_container.get_node("Bid1Btn").pressed.connect(_on_bid_pressed.bind(1))
	bid_container.get_node("Bid2Btn").pressed.connect(_on_bid_pressed.bind(2))
	bid_container.get_node("Bid3Btn").pressed.connect(_on_bid_pressed.bind(3))
	
	# Connect Play buttons
	play_container.get_node("PassPlayBtn").pressed.connect(_on_pass_pressed)
	play_container.get_node("PlayBtn").pressed.connect(_on_play_pressed)
	
	# Setup mocked seats for demo
	seat_me.is_my_seat = true
	seat_me.update_info("Player 1", false, "Undecided", 0)
	seat_left.update_info("Player 2", false, "Undecided", 0)
	seat_right.update_info("Player 3", false, "Undecided", 0)

func _update_ui_state(stage: String):
	game_stage = stage
	table_center.stage_label.text = "Stage: " + stage
	
	ready_btn.visible = (stage == "Waiting" or stage == "Finished")
	start_btn.visible = (stage == "Waiting") # Usually only if host and all ready
	bid_container.visible = (stage == "Bidding")
	play_container.visible = (stage == "Playing")

func _on_ready_pressed():
	seat_me.update_info("Player 1", true, "Undecided", 0)
	# Mock server event: everyone is ready
	seat_left.update_info("Player 2", true, "Undecided", 0)
	seat_right.update_info("Player 3", true, "Undecided", 0)

func _on_start_pressed():
	_update_ui_state("Bidding")
	_deal_cards_demo()

func _deal_cards_demo():
	# Give 17 cards to me for demo
	var my_cards = []
	for i in range(17):
		my_cards.append({"suit": i % 4, "rank": (i % 13) + 1})
	seat_me.set_hand(my_cards)
	
	seat_me.update_info("Player 1", false, "Undecided", 17)
	seat_left.update_info("Player 2", false, "Undecided", 17)
	seat_right.update_info("Player 3", false, "Undecided", 17)
	
	# 3 hole cards face down
	table_center.set_hole_cards([
		{"suit": 5, "rank": 0},
		{"suit": 5, "rank": 0},
		{"suit": 5, "rank": 0}
	])

# Buttons connected via Editor or code
func _on_bid_pressed(score: int):
	# Mock bid
	table_center.set_action("me", "Bid: " + str(score))
	_update_ui_state("Playing")
	# Mock landlord setup
	seat_me.update_info("Player 1", false, "Landlord", 20)
	
	# Reveal hole cards
	table_center.set_hole_cards([
		{"suit": 1, "rank": 5},
		{"suit": 2, "rank": 10},
		{"suit": 3, "rank": 1}
	])

func _on_play_pressed():
	var selected = seat_me.get_selected_cards()
	if selected.is_empty():
		return
	table_center.set_action("me", "Played " + str(selected.size()) + " cards")
	
	# Remove selected from hand for demo
	for i in range(selected.size() - 1, -1, -1):
		seat_me.hand_container.get_child(selected[i].index).queue_free()
		
func _on_pass_pressed():
	table_center.set_action("me", "Pass")
