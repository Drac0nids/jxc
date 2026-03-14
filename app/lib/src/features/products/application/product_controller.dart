import 'package:flutter/foundation.dart';

import '../../../core/models/user_role.dart';
import '../../../network/api_exception.dart';
import '../models/product_models.dart';
import '../models/product_repository.dart';

class ProductController extends ChangeNotifier {
  ProductController({
    required ProductRepository repository,
    required UserRole role,
  })  : _repository = repository,
        _role = role;

  final ProductRepository _repository;
  final UserRole _role;

  bool _loading = false;
  bool _submitting = false;

  String? _errorMessage;
  String? _successMessage;

  List<ProductData> _list = <ProductData>[];
  int _total = 0;
  int _page = 1;
  int _pageSize = 20;
  String _keyword = '';
  String _barcode = '';
  int? _categoryId;

  ProductData? _selected;

  bool get canWrite => _role == UserRole.owner || _role == UserRole.purchaser;
  bool get canViewCostPrice =>
      _role == UserRole.owner || _role == UserRole.purchaser;

  bool get loading => _loading;
  bool get submitting => _submitting;

  String? get errorMessage => _errorMessage;
  String? get successMessage => _successMessage;

  List<ProductData> get list => List<ProductData>.unmodifiable(_list);
  int get total => _total;
  int get page => _page;
  int get pageSize => _pageSize;
  String get keyword => _keyword;
  String get barcode => _barcode;
  ProductData? get selected => _selected;

  void clearMessages() {
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();
  }

  void clearSelected() {
    _selected = null;
    notifyListeners();
  }

  void selectProduct(ProductData? product) {
    _selected = product;
    notifyListeners();
  }

  Future<void> loadProducts({
    int page = 1,
    int pageSize = 20,
    String? keyword,
    String? barcode,
    int? categoryId,
  }) async {
    if (_loading) {
      return;
    }

    _loading = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      final query = ListProductsQuery(
        page: page,
        pageSize: pageSize,
        keyword: keyword ?? _keyword,
        barcode: barcode ?? _barcode,
        categoryId: categoryId,
      );

      final response = await _repository.listProducts(query);
      _list = response.list;
      _total = response.total;
      _page = response.page;
      _pageSize = response.pageSize;
      _keyword = (keyword ?? _keyword).trim();
      _barcode = (barcode ?? _barcode).trim();
      _categoryId = categoryId;
      if (_selected != null) {
        final index = _list.indexWhere((item) => item.id == _selected!.id);
        _selected = index >= 0 ? _list[index] : null;
      }
    } catch (error) {
      _errorMessage = _humanizeError(error);
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  Future<void> refresh() async {
    await loadProducts(
      page: _page,
      pageSize: _pageSize,
      keyword: _keyword,
      barcode: _barcode,
      categoryId: _categoryId,
    );
  }

  Future<ProductData?> createProduct(CreateProductRequest request) async {
    if (_submitting) {
      return null;
    }

    if (!canWrite) {
      _errorMessage = '当前角色无商品写权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    _submitting = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      final created = await _repository.createProduct(request);
      _selected = created;
      _successMessage = '商品创建成功：#${created.id} ${created.name}';
      await refresh();
      return created;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return null;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  Future<ProductData?> updateProduct({
    required int id,
    required UpdateProductRequest request,
  }) async {
    if (_submitting) {
      return null;
    }

    if (!canWrite) {
      _errorMessage = '当前角色无商品写权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    _submitting = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      final updated = await _repository.updateProduct(id: id, request: request);
      _selected = updated;
      _successMessage = '商品更新成功：#${updated.id} ${updated.name}';
      await refresh();
      return updated;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return null;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  Future<DeleteProductResult?> deleteProduct({
    required int id,
    int? expectedVersion,
  }) async {
    if (_submitting) {
      return null;
    }

    if (!canWrite) {
      _errorMessage = '当前角色无商品写权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    _submitting = true;
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();

    try {
      final deleted = await _repository.deleteProduct(
        id: id,
        expectedVersion: expectedVersion,
      );
      _selected = null;
      _successMessage = '商品删除成功：#$id';
      await refresh();
      return deleted;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return null;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  String _humanizeError(Object error) {
    if (error is ApiException) {
      if (error.code == 4091) {
        return '版本冲突，请刷新后重试（code=4091）';
      }
      if (error.code == 4090) {
        return '${error.message}（code=4090）';
      }
      if (error.code == 4002) {
        return '${error.message}（code=4002）';
      }
      if (error.code == 4030) {
        return '当前账号无权限执行该操作（code=4030）';
      }

      final requestIdPart =
          (error.requestId == null || error.requestId!.isEmpty)
              ? ''
              : '，request_id=${error.requestId}';
      return '${error.message}（code=${error.code}$requestIdPart）';
    }

    if (error is FormatException) {
      return error.message;
    }

    if (error is Exception) {
      return error.toString().replaceFirst('Exception: ', '');
    }

    return '商品操作失败，请稍后重试';
  }
}
