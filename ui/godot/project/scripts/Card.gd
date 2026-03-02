extends TextureRect
class_name Card

enum Suit { SPADES, HEARTS, DIAMONDS, CLUBS, JOKER, HIDDEN }

@export var suit: Suit = Suit.SPADES :
	set(value):
		suit = value
		_update_texture()

@export var rank: int = 1 :
	set(value):
		rank = value
		_update_texture()

@export var is_selected: bool = false :
	set(value):
		if is_selected != value:
			is_selected = value
			position.y = -20 if is_selected else 0

var card_size := Vector2(64, 64)
var card_spacing := 1

func _ready():
	_update_texture()

func _update_texture():
	if texture is AtlasTexture:
		var col = 0
		var row = 0
		
		if suit == Suit.HIDDEN:
			# Usually card back is somewhere like bottom right
			col = 13
			row = 3
		elif suit == Suit.JOKER:
			col = 13
			row = 0 if rank == 14 else 1 # 14=Black/Small Joker, 15=Red/Big Joker
		else:
			col = rank - 1 # rank 1=A->0, 2->1 ... 13=K->12
			row = suit # 0=Spades, 1=Hearts, etc.
			
		var x = col * (card_size.x + card_spacing)
		var y = row * (card_size.y + card_spacing)
		
		texture.region = Rect2(Vector2(x, y), card_size)

func _gui_input(event):
	if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT and event.pressed:
		is_selected = !is_selected
