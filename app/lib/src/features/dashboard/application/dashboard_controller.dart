import 'package:flutter/foundation.dart';

import '../../../network/api_exception.dart';
import '../../../network/exception_utils.dart';
import '../models/dashboard_data.dart';
import '../models/dashboard_repository.dart';

class DashboardController extends ChangeNotifier {
  DashboardController({required DashboardRepository repository})
      : _repository = repository;

  final DashboardRepository _repository;

  DashboardData? _data;
  bool _loading = false;
  String? _errorMessage;
  String _selectedDate = '';

  DashboardData? get data => _data;
  bool get loading => _loading;
  String? get errorMessage => _errorMessage;
  String get selectedDate => _selectedDate;

  Future<void> load({String? date}) async {
    if (_loading) {
      return;
    }

    if (date == null) {
      _selectedDate = '';
    } else {
      _selectedDate = date.trim();
    }

    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _data = await _repository.fetchDashboard(
        date: _selectedDate.isEmpty ? null : _selectedDate,
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
