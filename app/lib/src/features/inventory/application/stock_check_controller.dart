import 'package:flutter/foundation.dart';

import '../../../core/models/user_role.dart';
import '../../../network/api_exception.dart';
import '../models/inventory_models.dart';
import '../models/inventory_repository.dart';

class StockCheckCreateItemInput {
  const StockCheckCreateItemInput({required this.productId});

  final String productId;

  StockCheckCreateItemInput copyWith({String? productId}) {
    return StockCheckCreateItemInput(productId: productId ?? this.productId);
  }
}

class StockCheckConfirmItemInput {
  const StockCheckConfirmItemInput({
    required this.productId,
    required this.bookStock,
    required this.actualStock,
  });

  final int productId;
  final int bookStock;
  final String actualStock;

  StockCheckConfirmItemInput copyWith({
    int? productId,
    int? bookStock,
    String? actualStock,
  }) {
    return StockCheckConfirmItemInput(
      productId: productId ?? this.productId,
      bookStock: bookStock ?? this.bookStock,
      actualStock: actualStock ?? this.actualStock,
    );
  }
}

class StockCheckScanResult {
  const StockCheckScanResult({
    required this.items,
    required this.message,
  });

  final List<StockCheckCreateItemInput> items;
  final String message;
}

class StockCheckController extends ChangeNotifier {
  StockCheckController({
    required InventoryRepository repository,
    required UserRole role,
  })  : _repository = repository,
        _role = role;

  final InventoryRepository _repository;
  final UserRole _role;

  bool _loadingCreate = false;
  bool _loadingQuery = false;
  bool _loadingStart = false;
  bool _loadingConfirm = false;
  bool _loadingScan = false;

  String? _errorMessage;
  String? _successMessage;
  String? _scanErrorMessage;
  String? _scanSuccessMessage;
  StockCheckData? _result;
  List<StockCheckConfirmItemInput> _confirmItems =
      <StockCheckConfirmItemInput>[];

  bool get canOperate => _role == UserRole.owner || _role == UserRole.purchaser;
  bool get loadingCreate => _loadingCreate;
  bool get loadingQuery => _loadingQuery;
  bool get loadingStart => _loadingStart;
  bool get loadingConfirm => _loadingConfirm;
  bool get loadingScan => _loadingScan;
  bool get busy =>
      _loadingCreate ||
      _loadingQuery ||
      _loadingStart ||
      _loadingConfirm ||
      _loadingScan;

  String? get errorMessage => _errorMessage;
  String? get successMessage => _successMessage;
  String? get scanErrorMessage => _scanErrorMessage;
  String? get scanSuccessMessage => _scanSuccessMessage;
  StockCheckData? get result => _result;
  List<StockCheckConfirmItemInput> get confirmItems =>
      List<StockCheckConfirmItemInput>.unmodifiable(_confirmItems);

  String get suggestedStartExpectedVersion => _result?.version.toString() ?? '';

  void clearMessages() {
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();
  }

  void clearScanMessages() {
    _scanErrorMessage = null;
    _scanSuccessMessage = null;
    notifyListeners();
  }

  void updateConfirmActualStock(
      {required int productId, required String actualStock}) {
    final index =
        _confirmItems.indexWhere((item) => item.productId == productId);
    if (index < 0) {
      return;
    }

    _confirmItems[index] =
        _confirmItems[index].copyWith(actualStock: actualStock);
    notifyListeners();
  }

  String? validateCreateForm({
    required List<StockCheckCreateItemInput> items,
  }) {
    if (!canOperate) {
      return '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作';
    }

    if (items.isEmpty) {
      return '请至少填写一条盘点明细';
    }

    final seen = <int>{};
    for (int index = 0; index < items.length; index += 1) {
      final row = index + 1;
      final parsedProductId = _parsePositiveInt(items[index].productId);
      if (parsedProductId == null) {
        return '第 $row 行商品ID必须为正整数';
      }
      if (seen.contains(parsedProductId)) {
        return '第 $row 行商品ID重复，请检查';
      }
      seen.add(parsedProductId);
    }

    return null;
  }

  String? validateQueryForm({required String id}) {
    final parsedId = _parsePositiveInt(id);
    if (parsedId == null) {
      return '请输入正确的盘点单ID（正整数）';
    }
    return null;
  }

  String? validateStartForm({
    required String expectedVersion,
  }) {
    if (!canOperate) {
      return '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作';
    }

    if (_result == null) {
      return '请先查询盘点单';
    }

    if (_result!.status != 'DRAFT') {
      return '当前状态为 ${_result!.status}，仅 DRAFT 状态可开始盘点';
    }

    if (expectedVersion.trim().isNotEmpty &&
        _parseNonNegativeInt(expectedVersion) == null) {
      return '开始盘点版本必须为大于等于 0 的整数';
    }

    return null;
  }

  String? validateConfirmForm({required String expectedVersion}) {
    if (!canOperate) {
      return '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作';
    }

    if (_result == null) {
      return '请先查询盘点单';
    }

    if (_result!.status != 'COUNTING') {
      return '当前状态为 ${_result!.status}，仅 COUNTING 状态可确认盘点';
    }

    if (expectedVersion.trim().isNotEmpty &&
        _parseNonNegativeInt(expectedVersion) == null) {
      return '确认盘点版本必须为大于等于 0 的整数';
    }

    if (_confirmItems.isEmpty) {
      return '盘点确认明细不能为空，请重新查询后重试';
    }

    final expectedProductIds =
        _result!.items.map((item) => item.productId).toSet();
    final seen = <int>{};

    for (int index = 0; index < _confirmItems.length; index += 1) {
      final row = index + 1;
      final item = _confirmItems[index];

      if (seen.contains(item.productId)) {
        return '第 $row 行商品ID重复，请检查';
      }
      seen.add(item.productId);

      if (!expectedProductIds.contains(item.productId)) {
        return '第 $row 行商品ID不在盘点单明细中';
      }

      final actualStock = _parseNonNegativeInt(item.actualStock);
      if (actualStock == null) {
        return '第 $row 行实盘库存必须为大于等于 0 的整数';
      }
    }

    if (seen.length != expectedProductIds.length) {
      return '确认明细与盘点单明细不一致，请重新查询后再确认';
    }

    return null;
  }

  Future<StockCheckScanResult?> scanAndAppendCreateItem({
    required String barcode,
    required List<StockCheckCreateItemInput> items,
  }) async {
    if (busy) {
      return null;
    }

    _scanErrorMessage = null;
    _scanSuccessMessage = null;

    if (!canOperate) {
      _scanErrorMessage = '当前角色无库存盘点操作权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    final barcodeValue = barcode.trim();
    if (barcodeValue.isEmpty) {
      _scanErrorMessage = '请输入条码后再扫码';
      notifyListeners();
      return null;
    }

    _loadingScan = true;
    notifyListeners();

    try {
      final scanned = await _repository.scanProduct(barcodeValue);
      final nextItems = items.map((item) => item.copyWith()).toList();
      final exists = nextItems.any(
        (item) => _parsePositiveInt(item.productId) == scanned.id,
      );

      if (exists) {
        final message = '商品 #${scanned.id} ${scanned.name} 已在盘点明细中，无需重复添加';
        _scanSuccessMessage = message;
        return StockCheckScanResult(items: nextItems, message: message);
      }

      nextItems
          .add(StockCheckCreateItemInput(productId: scanned.id.toString()));
      final message = '扫码成功：#${scanned.id} ${scanned.name}，已加入盘点明细';
      _scanSuccessMessage = message;
      return StockCheckScanResult(items: nextItems, message: message);
    } catch (error) {
      _scanErrorMessage = _humanizeError(error, forScan: true);
      return null;
    } finally {
      _loadingScan = false;
      notifyListeners();
    }
  }

  Future<bool> createStockCheck({
    required String remark,
    required List<StockCheckCreateItemInput> items,
  }) async {
    if (busy) {
      return false;
    }

    clearMessages();

    final validationError = validateCreateForm(items: items);
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    final requestItems = items
        .map((item) => StockCheckCreateItemRequest(
            productId: _parsePositiveInt(item.productId.trim())!))
        .toList();

    final request = StockCheckCreateRequest(
      items: requestItems,
      remark: remark.trim().isEmpty ? null : remark.trim(),
    );

    _loadingCreate = true;
    notifyListeners();

    try {
      final response = await _repository.createStockCheck(request);
      _syncCheckContext(response);
      _successMessage = '盘点单创建成功：#${response.id}（${response.bizNo}）';
      clearScanMessages();
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _loadingCreate = false;
      notifyListeners();
    }
  }

  Future<bool> queryStockCheck({required String id}) async {
    if (busy) {
      return false;
    }

    clearMessages();

    final validationError = validateQueryForm(id: id);
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    final checkId = _parsePositiveInt(id.trim())!;

    _loadingQuery = true;
    notifyListeners();

    try {
      final response = await _repository.getStockCheck(checkId);
      _syncCheckContext(response);
      _successMessage = '盘点单查询成功：#${response.id}（状态：${response.status}）';
      clearScanMessages();
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _loadingQuery = false;
      notifyListeners();
    }
  }

  Future<bool> startStockCheck({required String expectedVersion}) async {
    if (busy) {
      return false;
    }

    clearMessages();

    final validationError = validateStartForm(expectedVersion: expectedVersion);
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    final checkId = _result!.id;
    final parsedExpectedVersion = expectedVersion.trim().isEmpty
        ? null
        : _parseNonNegativeInt(expectedVersion.trim());

    _loadingStart = true;
    notifyListeners();

    try {
      final response = await _repository.startStockCheck(
        id: checkId,
        expectedVersion: parsedExpectedVersion,
      );
      _syncCheckContext(response);
      _successMessage = '盘点单开始成功：#${response.id}（状态：${response.status}）';
      clearScanMessages();
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _loadingStart = false;
      notifyListeners();
    }
  }

  Future<bool> confirmStockCheck({
    required String expectedVersion,
    required String remark,
  }) async {
    if (busy) {
      return false;
    }

    clearMessages();

    final validationError =
        validateConfirmForm(expectedVersion: expectedVersion);
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    final checkId = _result!.id;
    final parsedExpectedVersion = expectedVersion.trim().isEmpty
        ? null
        : _parseNonNegativeInt(expectedVersion.trim());

    final request = StockCheckConfirmRequest(
      expectedVersion: parsedExpectedVersion,
      items: _confirmItems
          .map(
            (item) => StockCheckConfirmItemRequest(
              productId: item.productId,
              actualStock: _parseNonNegativeInt(item.actualStock.trim())!,
            ),
          )
          .toList(),
      remark: remark.trim().isEmpty ? null : remark.trim(),
    );

    _loadingConfirm = true;
    notifyListeners();

    try {
      final response =
          await _repository.confirmStockCheck(id: checkId, request: request);
      _syncCheckContext(response);
      _successMessage = '盘点单确认成功：#${response.id}（状态：${response.status}）';
      clearScanMessages();
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _loadingConfirm = false;
      notifyListeners();
    }
  }

  void _syncCheckContext(StockCheckData data) {
    _result = data;
    _confirmItems = data.items
        .map(
          (item) => StockCheckConfirmItemInput(
            productId: item.productId,
            bookStock: item.bookStock,
            actualStock: (item.actualStock ?? item.bookStock).toString(),
          ),
        )
        .toList();
  }

  String _humanizeError(Object error, {bool forScan = false}) {
    if (error is ApiException) {
      if (forScan && error.code == 4040) {
        return '未找到该条码对应商品，请先前往“商品管理”建档。';
      }
      if (error.code == 4091) {
        return '版本冲突，请刷新后重试（code=4091）';
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

    return forScan ? '扫码选品失败，请稍后重试' : '盘点操作失败，请稍后重试';
  }

  int? _parsePositiveInt(String raw) {
    final value = int.tryParse(raw.trim());
    if (value == null || value <= 0) {
      return null;
    }

    return value;
  }

  int? _parseNonNegativeInt(String raw) {
    final value = int.tryParse(raw.trim());
    if (value == null || value < 0) {
      return null;
    }

    return value;
  }
}
