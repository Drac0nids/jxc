import 'package:flutter/material.dart';
import '../models/category_models.dart';
import '../models/category_repository.dart';

class CategoryController extends ChangeNotifier {
  CategoryController({required CategoryRepository repository})
      : _repository = repository;

  final CategoryRepository _repository;

  List<CategoryNode> _tree = [];
  bool _loading = false;
  String? _error;

  List<CategoryNode> get tree => _tree;
  bool get loading => _loading;
  String? get error => _error;

  bool get isEmpty => _tree.isEmpty;

  /// 获取树形分类（所有节点展平方便查 id→name）
  List<CategoryNode> get flatAll {
    final result = <CategoryNode>[];
    void addAll(List<CategoryNode> nodes) {
      for (final n in nodes) {
        result.add(n);
        addAll(n.children);
      }
    }

    addAll(_tree);
    return result;
  }

  CategoryNode? findById(int id) {
    for (final n in flatAll) {
      if (n.id == id) return n;
    }
    return null;
  }

  /// 按分类 id 拼接路径字符串（如：食品饮料 > 碳酸饮料 > 可乐）
  String? buildPath(int? categoryId) {
    if (categoryId == null) return null;
    final node = findById(categoryId);
    if (node == null) return null;

    // 通过 level 向上追溯父节点
    String path = node.name;
    if (node.level > 1) {
      // 在 flatAll 中找父节点
      for (final parent in flatAll) {
        for (final child in parent.children) {
          if (child.id == node.id) {
            final grandPath = buildPath(parent.id);
            path = grandPath != null ? '$grandPath > $path' : path;
            return path;
          }
        }
      }
    }
    return path;
  }

  Future<void> load() async {
    _loading = true;
    _error = null;
    notifyListeners();

    try {
      _tree = await _repository.listCategoryTree();
      _error = null;
    } catch (e) {
      _error = '加载分类失败：$e';
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<CategoryNode?> createCategory(CreateCategoryRequest req) async {
    try {
      final created = await _repository.createCategory(req);
      await load(); // 刷新树
      return created;
    } catch (e) {
      _error = '创建分类失败：$e';
      notifyListeners();
      return null;
    }
  }

  Future<bool> updateCategory(int id, UpdateCategoryRequest req) async {
    try {
      await _repository.updateCategory(id, req);
      await load();
      return true;
    } catch (e) {
      _error = '更新分类失败：$e';
      notifyListeners();
      return false;
    }
  }

  Future<bool> deleteCategory(int id) async {
    try {
      await _repository.deleteCategory(id);
      await load();
      return true;
    } catch (e) {
      _error = '删除分类失败：$e';
      notifyListeners();
      return false;
    }
  }
}
