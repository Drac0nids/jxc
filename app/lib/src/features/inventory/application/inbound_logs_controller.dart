import 'package:flutter/foundation.dart';

import '../../../core/models/user_role.dart';
import '../../../network/api_exception.dart';
import '../../../network/exception_utils.dart';
import '../models/inventory_models.dart';
import '../models/inventory_repository.dart';

class InboundLogsController extends ChangeNotifier {
  InboundLogsController({
    required InventoryRepository repository,
    required UserRole role,
  })  : _repository = repository,
        _role = role;

  static final RegExp _datePattern = RegExp(r'^\d{4}-\d{2}-\d{2}$');

  final InventoryRepository _repository;
  final UserRole _role;

  bool _loading = false;
  String? _errorMessage;
  InboundLogsPageData? _data;

  bool get canView => _role == UserRole.owner || _role == UserRole.purchaser;
  bool get loading => _loading;
  String? get errorMessage => _errorMessage;
  InboundLogsPageData? get data => _data;

  String? validateQuery({
    required String startDate,
    required String endDate,
    required int page,
    required int pageSize,
  }) {
    if (!canView) {
      return '当前角色无入库记录查询权限，仅 OWNER/PURCHASER 可查看';
    }

    final start = startDate.trim();
    final end = endDate.trim();
    if (!_datePattern.hasMatch(start) || !_datePattern.hasMatch(end)) {
      return '日期格式必须为 YYYY-MM-DD';
    }

    final startParsed = _parseDateStrict(start);
    final endParsed = _parseDateStrict(end);
    if (startParsed == null || endParsed == null) {
      return '日期不合法，请重新选择';
    }
    if (startParsed.isAfter(endParsed)) {
      return '开始日期不能晚于结束日期';
    }

    if (page <= 0) {
      return '页码必须为正整数';
    }
    if (pageSize < 1 || pageSize > 100) {
      return '每页条数必须为 1 ~ 100 的整数';
    }

    return null;
  }

  Future<bool> load({
    required String startDate,
    required String endDate,
    required int page,
    required int pageSize,
  }) async {
    if (_loading) {
      return false;
    }

    final validationError = validateQuery(
      startDate: startDate,
      endDate: endDate,
      page: page,
      pageSize: pageSize,
    );
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    _loading = true;
    _errorMessage = null;
    notifyListeners();

    try {
      _data = await _repository.fetchInboundLogs(
        startDate: startDate.trim(),
        endDate: endDate.trim(),
        page: page,
        pageSize: pageSize,
      );
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _loading = false;
      notifyListeners();
    }
  }

  DateTime? _parseDateStrict(String raw) {
    final parsed = DateTime.tryParse(raw);
    if (parsed == null) {
      return null;
    }

    final normalized =
        '${parsed.year.toString().padLeft(4, '0')}-${parsed.month.toString().padLeft(2, '0')}-${parsed.day.toString().padLeft(2, '0')}';
    if (normalized != raw) {
      return null;
    }

    return parsed;
  }

  String _humanizeError(Object error) => humanizeError(error);
}
