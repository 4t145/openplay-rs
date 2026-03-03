# file: res://scripts/resources/CardTheme.gd
class_name CardTheme
extends Resource

# 皮肤的图集纹理
@export var texture: Texture2D
# 单张卡片的宽度
@export var card_width: float = 64.0
# 单张卡片的高度
@export var card_height: float = 64.0
# 纹理之间的间距（如果有）
@export var h_separation: float = 0.0
@export var v_separation: float = 0.0

# 根据花色和点数计算 Atlas 的区域
func get_card_region(suit: int, rank: int) -> Rect2:
    # 假设图集排列方式：
    # 行（Y轴）：花色 (0:黑桃, 1:红桃, 2:梅花, 3:方片)
    # 列（X轴）：点数 (0-12, 对应 A-K)
    # 注意：这里的逻辑需要根据你实际的图片排列调整
    # 将 rank 转换为 0-based 索引 (假设 rank 传入 1-13)
    var rank_idx = rank - 1
    
    var x = rank_idx * (card_width + h_separation)
    var y = suit * (card_height + v_separation)
    
    return Rect2(x, y, card_width, card_height)