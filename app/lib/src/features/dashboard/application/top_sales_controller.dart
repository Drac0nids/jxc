import 'package:flutter/foundation.dart';

import '../../../network/api_exception.dart';
import '../models/dashboard_data.dart';
import '../models/dashboard_repository.dart';

enum TopSalesSortMode { qty, sales, profit }

class TopSalesController extends ChangeNotifier {
  TopSalesController({required DashboardRepository repository})
      : _repository = repository;

  final DashboardRepository _repository;

  bool _loading = false;
  String? _errorMessage;
  SalesReportData? _data;
  TopSalesSortMode _sortMode = TopSalesSortMode.qty;

  bool get loading => _loading;
  String? get errorMessage => _errorMessage;
  SalesReportData? get data => _data;
  TopSalesSortMode get sortMode => _sortMode;

  /// Items sorted by current mode.
  List<SalesReportItemData> get sortedItems {
    if (_data == null) return const <SalesReportItemData>[];
    final list = List<SalesReportItemData>.from(_data!.list);
    switch (_sortMode) {
      case TopSalesSortMode.qty:
        list.sort((a, b) => b.totalQty.compareTo(a.totalQty));
      case TopSalesSortMode.sales:
        list.sort((a, b) => b.totalSalesDouble.compareTo(a.totalSalesDouble));
      case TopSalesSortMode.profit:
        list.sort(
            (a, b) => b.grossProfitDouble.compareTo(a.grossProfitDouble));
    }
    return list;
  }

  void setSortMode(TopSalesSortMode mode) {
    if (_sortMode == mode) return;
    _sortMode = mode;
    notifyListeners();
  }

  Future<void> load({
    required String startDate,
    required String endDate,
  }) async {
    if (_loading) return;

    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _data = await _repository.fetchSalesReport(
        startDate: startDate,
        endDate: endDate,
        pageSize: 100,
      );
    } catch (error) {
      if (error is ApiException) {
        _errorMessage = '${error.message}（code=${error.code}）';
      } else if (error is FormatException) {
        _errorMessage = error.message;
      } else {
        _errorMessage = '加载销售排行失败，请稍后重试';
      }
    } finally {
      _loading = false;
      notifyListeners();
    }
  }
}
