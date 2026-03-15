import 'package:flutter/foundation.dart';

import '../../../network/api_exception.dart';
import '../../../network/exception_utils.dart';
import '../models/product_models.dart';
import '../models/product_repository.dart';

/// 低库存商品控制器 — 仅用于看板低库存下钻，只读
class LowStockController extends ChangeNotifier {
  LowStockController({required ProductRepository repository})
      : _repository = repository;

  final ProductRepository _repository;

  List<ProductData> _items = const <ProductData>[];
  int _total = 0;
  bool _loading = false;
  String? _errorMessage;

  List<ProductData> get items => _items;
  int get total => _total;
  bool get loading => _loading;
  String? get errorMessage => _errorMessage;

  Future<void> load() async {
    if (_loading) return;

    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      final result = await _repository.listProducts(
        const ListProductsQuery(
          lowStock: true,
          pageSize: 100,
        ),
      );
      _items = result.list;
      // Sort by gap descending (largest deficit first) on client side
      _items.sort((a, b) {
        final gapA = a.minStockLimit - a.currentStock;
        final gapB = b.minStockLimit - b.currentStock;
        return gapB.compareTo(gapA);
      });
      _total = result.total > 0 ? result.total : _items.length;
    } catch (error) {
      _errorMessage = _humanizeError(error);
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  String _humanizeError(Object error) => humanizeError(error);
}
