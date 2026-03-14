/// 分类节点（树形结构）
class CategoryNode {
  const CategoryNode({
    required this.id,
    required this.name,
    required this.level,
    required this.sortOrder,
    required this.children,
  });

  final int id;
  final String name;
  final int level;
  final int sortOrder;
  final List<CategoryNode> children;

  factory CategoryNode.fromJson(Map<String, dynamic> json) {
    final childrenRaw = json['children'];
    return CategoryNode(
      id: _toInt(json['id']),
      name: (json['name'] ?? '').toString(),
      level: _toInt(json['level']),
      sortOrder: _toInt(json['sort_order']),
      children: (childrenRaw is List)
          ? childrenRaw
              .whereType<Map<String, dynamic>>()
              .map(CategoryNode.fromJson)
              .toList()
          : <CategoryNode>[],
    );
  }

  /// 构建路径字符串，如：食品饮料 > 碳酸饮料 > 可乐
  String get path => name;
}

class CreateCategoryRequest {
  const CreateCategoryRequest({
    required this.name,
    this.parentId,
    this.sortOrder,
  });

  final String name;
  final int? parentId;
  final int? sortOrder;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'name': name.trim(),
        if (parentId != null) 'parent_id': parentId,
        if (sortOrder != null) 'sort_order': sortOrder,
      };
}

class UpdateCategoryRequest {
  const UpdateCategoryRequest({this.name, this.sortOrder});

  final String? name;
  final int? sortOrder;

  Map<String, dynamic> toJson() => <String, dynamic>{
        if (name != null && name!.trim().isNotEmpty) 'name': name!.trim(),
        if (sortOrder != null) 'sort_order': sortOrder,
      };
}

int _toInt(Object? value) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  if (value is String) return int.tryParse(value) ?? 0;
  return 0;
}
