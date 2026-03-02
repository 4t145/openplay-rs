extends Control

@onready var title_label: Label = $Root/TitleLabel
@onready var hint_label: Label = $Root/HintLabel
@onready var user_list: ItemList = $Root/Content/UserList
@onready var name_label: Label = $Root/Content/Details/NameLabel
@onready var id_label: Label = $Root/Content/Details/IdLabel
@onready var select_button: Button = $Root/Actions/SelectButton
@onready var create_button: Button = $Root/Actions/CreateButton
@onready var delete_button: Button = $Root/Actions/DeleteButton
@onready var refresh_button: Button = $Root/Actions/RefreshButton
@onready var status_label: Label = $Root/StatusLabel

const GAME_SCENE := "res://scenes/Game.tscn"

var strings := preload("res://scripts/strings.gd").new()
var sdk: Object = null
var identities: Array = []
var current_index: int = -1

func _ready() -> void:
    title_label.text = strings.get_text("title")
    hint_label.text = strings.get_text("user_select_hint")
    select_button.text = strings.get_text("select")
    create_button.text = strings.get_text("create")
    delete_button.text = strings.get_text("delete")
    refresh_button.text = strings.get_text("refresh")

    user_list.item_selected.connect(_on_item_selected)
    select_button.pressed.connect(_on_select_pressed)
    create_button.pressed.connect(_on_create_pressed)
    delete_button.pressed.connect(_on_delete_pressed)
    refresh_button.pressed.connect(_on_refresh_pressed)

    _init_sdk()
    _reload_identities()

func _init_sdk() -> void:
    if ClassDB.class_exists("OpenPlaySdkRef"):
        sdk = ClassDB.instantiate("OpenPlaySdkRef")
    else:
        status_label.text = strings.get_text("sdk_not_loaded")

func _reload_identities() -> void:
    user_list.clear()
    identities.clear()
    current_index = -1

    if sdk == null:
        return

    var list = sdk.call("list_identities")
    if typeof(list) != TYPE_ARRAY:
        return

    for entry in list:
        identities.append(entry)
        var nickname = str(entry.get("nickname", ""))
        var user_id = str(entry.get("user_id", ""))
        var label = "[头像] %s  %s" % [nickname, _truncate_id(user_id)]
        user_list.add_item(label)

    if identities.size() > 0:
        user_list.select(0)
        _apply_selection(0)

func _apply_selection(index: int) -> void:
    current_index = index
    var entry = identities[index]
    name_label.text = "%s" % str(entry.get("nickname", ""))
    id_label.text = "%s" % str(entry.get("user_id", ""))
    status_label.text = ""

func _on_item_selected(index: int) -> void:
    if index >= 0 and index < identities.size():
        _apply_selection(index)

func _on_select_pressed() -> void:
    if current_index < 0:
        status_label.text = strings.get_text("select_empty")
        return
    var entry = identities[current_index]
    status_label.text = strings.get_text("selected_prefix") + str(entry.get("nickname", ""))
    get_tree().change_scene_to_file(GAME_SCENE)

func _on_create_pressed() -> void:
    if sdk == null:
        return
    var title = strings.get_text("create_title")
    var hint = strings.get_text("create_hint")
    var nickname = str(await _request_text(title, hint, strings.get_text("default_nickname")))
    if nickname.strip_edges() == "":
        status_label.text = strings.get_text("create_failed")
        return
    var created = sdk.call("create_identity", nickname)
    if typeof(created) == TYPE_DICTIONARY and created.size() > 0:
        status_label.text = strings.get_text("created")
        _reload_identities()
    else:
        status_label.text = strings.get_text("create_failed")

func _on_delete_pressed() -> void:
    if sdk == null:
        return
    if current_index < 0:
        status_label.text = strings.get_text("delete_empty")
        return
    var entry = identities[current_index]
    var path = str(entry.get("path", ""))
    if path == "":
        status_label.text = strings.get_text("delete_failed")
        return
    var ok = false
    var confirm = await _request_confirm(strings.get_text("delete_title"), strings.get_text("delete_hint"))
    if confirm:
        ok = sdk.call("delete_identity", path)
    if ok:
        status_label.text = strings.get_text("deleted")
        _reload_identities()
    else:
        status_label.text = strings.get_text("delete_failed")

func _on_refresh_pressed() -> void:
    _reload_identities()

func _truncate_id(user_id: String) -> String:
    if user_id.length() <= 16:
        return user_id
    return "%s..%s" % [user_id.substr(0, 8), user_id.substr(user_id.length() - 6, 6)]

func _request_text(title: String, hint: String, default_value: String) -> String:
    var dialog = AcceptDialog.new()
    dialog.title = title
    dialog.dialog_text = hint
    var line_edit = LineEdit.new()
    line_edit.text = default_value
    dialog.add_child(line_edit)
    add_child(dialog)

    var value = ""
    dialog.confirmed.connect(func():
        value = line_edit.text
        dialog.queue_free()
    )
    dialog.canceled.connect(func():
        value = ""
        dialog.queue_free()
    )

    dialog.popup_centered()
    await dialog.tree_exited
    return value

func _request_confirm(title: String, hint: String) -> bool:
    var dialog = ConfirmationDialog.new()
    dialog.title = title
    dialog.dialog_text = hint
    add_child(dialog)

    var confirmed = false
    dialog.confirmed.connect(func():
        confirmed = true
        dialog.queue_free()
    )
    dialog.canceled.connect(func():
        confirmed = false
        dialog.queue_free()
    )

    dialog.popup_centered()
    await dialog.tree_exited
    return confirmed
