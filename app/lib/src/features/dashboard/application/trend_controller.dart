import 'package:flutter/foundation.dart';

import '../../../network/api_exception.dart';
import '../../../network/exception_utils.dart';
import '../models/dashboard_data.dart';
import '../models/dashboard_repository.dart';

class TrendController extends ChangeNotifier {
  TrendController({required DashboardRepository repository})
      : _repository = repository;

  final DashboardRepository _repository;

  TrendData? _data;
  bool _loading = false;
  String? _errorMessage;

  TrendData? get data => _data;
  bool get loading => _loading;
  String? get errorMessage => _errorMessage;

  Future<void> load({
    required String startDate,
    required String endDate,
  }) async {
    if (_loading) return;

    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _data = await _repository.fetchTrend(
        startDate: startDate,
        endDate: endDate,
      );
    } catch (error) {
      _errorMessage = _humanizeError(error);
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  String _humanizeError(Object error) => humanizeError(error);
}
