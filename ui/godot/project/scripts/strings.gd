extends Node

var _texts := {
    "title": "OpenPlay",
    "user_select_hint": "请选择用户",
    "direct": "直连",
    "steam": "Steam",
    "status_prefix": "SDK 版本: ",
    "status_direct": "已选择: 直连",
    "status_steam": "已选择: Steam",
    "select": "选择",
    "create": "新建",
    "delete": "删除",
    "refresh": "刷新",
    "sdk_not_loaded": "SDK 未加载",
    "select_empty": "请选择一个用户",
    "created": "已创建用户",
    "create_failed": "创建失败",
    "create_title": "创建用户",
    "create_hint": "输入昵称",
    "delete_empty": "没有可删除的用户",
    "deleted": "已删除用户",
    "delete_failed": "删除失败",
    "delete_title": "删除用户",
    "delete_hint": "确定删除当前用户？",
    "selected_prefix": "已选择: ",
    "default_nickname": "player",
}

func get_text(key: String) -> String:
    return _texts.get(key, key)
