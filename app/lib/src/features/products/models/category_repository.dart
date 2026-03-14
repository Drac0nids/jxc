import '../../../network/api_client.dart';
import 'category_models.dart';

class CategoryRepository {
  const CategoryRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  /// 获取完整分类树
  Future<List<CategoryNode>> listCategoryTree() async {
    final envelope = await _client.get<List<CategoryNode>>(
      '/categories/tree',
      decoder: (Object? rawData) {
        if (rawData is! List) {
          throw const FormatException('category tree response data is invalid');
        }
        return rawData
            .whereType<Map<String, dynamic>>()
            .map(CategoryNode.fromJson)
            .toList();
      },
    );
    return envelope.data;
  }

  /// 创建分类
  Future<CategoryNode> createCategory(CreateCategoryRequest req) async {
    final envelope = await _client.post<CategoryNode>(
      '/categories',
      data: req.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('create category response data is invalid');
        }
        return CategoryNode.fromJson(rawData);
      },
    );
    return envelope.data;
  }

  /// 更新分类名/排序
  Future<CategoryNode> updateCategory(int id, UpdateCategoryRequest req) async {
    final envelope = await _client.put<CategoryNode>(
      '/categories/$id',
      data: req.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('update category response data is invalid');
        }
        return CategoryNode.fromJson(rawData);
      },
    );
    return envelope.data;
  }

  /// 删除分类
  Future<void> deleteCategory(int id) async {
    await _client.delete<void>(
      '/categories/$id',
      decoder: (_) {},
    );
  }
}
