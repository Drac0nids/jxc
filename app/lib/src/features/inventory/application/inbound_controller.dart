import 'package:flutter/foundation.dart';

import '../../../core/models/user_role.dart';
import '../../../network/api_exception.dart';
import '../models/inventory_models.dart';
import '../models/inventory_repository.dart';

class InboundScanResult {
  const InboundScanResult({
    required this.productIdText,
    required this.productName,
    required this.barcodeText,
    required this.qtyText,
    required this.unitCostText,
    required this.message,
  });

  final String productIdText;
  final String productName;
  final String barcodeText;
  final String qtyText;
  final String unitCostText;
  final String message;
}

class InboundScanPreview {
  const InboundScanPreview({
    required this.productIdText,
    required this.productName,
    required this.barcodeText,
    required this.unitCostText,
  });

  final String productIdText;
  final String productName;
  final String barcodeText;
  final String unitCostText;
}

class InboundBatchFormItemInput {
  const InboundBatchFormItemInput({
    required this.productId,
    required this.barcode,
    required this.qty,
    required this.unitCost,
    required this.expectedVersion,
    required this.remark,
  });

  final String productId;
  final String barcode;
  final String qty;
  final String unitCost;
  final String expectedVersion;
  final String remark;
}

class InboundController extends ChangeNotifier {
  InboundController({
    required InventoryRepository repository,
    required UserRole role,
  })  : _repository = repository,
        _role = role;

  final InventoryRepository _repository;
  final UserRole _role;

  bool _submitting = false;
  bool _scanning = false;
  String? _errorMessage;
  String? _successMessage;
  String? _scanErrorMessage;
  String? _scanSuccessMessage;
  String? _pendingMissingBarcode;
  InboundResultData? _result;
  InboundBatchResultData? _batchResult;

  bool get canSubmit => _role == UserRole.owner || _role == UserRole.purchaser;
  bool get submitting => _submitting;
  bool get scanning => _scanning;
  String? get errorMessage => _errorMessage;
  String? get successMessage => _successMessage;
  String? get scanErrorMessage => _scanErrorMessage;
  String? get scanSuccessMessage => _scanSuccessMessage;
  String? get pendingMissingBarcode => _pendingMissingBarcode;
  InboundResultData? get result => _result;
  InboundBatchResultData? get batchResult => _batchResult;

  String? consumePendingMissingBarcode() {
    final barcode = _pendingMissingBarcode;
    _pendingMissingBarcode = null;
    return barcode;
  }

  void clearMessages() {
    _errorMessage = null;
    _successMessage = null;
    notifyListeners();
  }

  void clearScanMessages() {
    _scanErrorMessage = null;
    _scanSuccessMessage = null;
    _pendingMissingBarcode = null;
    notifyListeners();
  }

  void clearResult() {
    _result = null;
    _batchResult = null;
    notifyListeners();
  }

  String? validateForm({
    required String productId,
    required String barcode,
    required String qty,
    required String unitCost,
    required String expectedVersion,
  }) {
    if (!canSubmit) {
      return '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作';
    }

    if (productId.trim().isEmpty && barcode.trim().isEmpty) {
      return 'product_id 或 barcode 至少填写一个';
    }

    if (productId.trim().isNotEmpty && _parsePositiveInt(productId) == null) {
      return '商品ID必须为正整数';
    }

    if (_parsePositiveInt(qty) == null) {
      return '数量必须为正整数';
    }

    if (unitCost.trim().isNotEmpty && !_isPriceValid(unitCost.trim())) {
      return '进货价格式错误（示例：2.20）';
    }

    if (expectedVersion.trim().isNotEmpty &&
        _parseNonNegativeInt(expectedVersion) == null) {
      return '版本必须为大于等于 0 的整数';
    }

    return null;
  }

  Future<InboundScanResult?> scanAndAccumulate({
    required String barcode,
    required String currentProductId,
    required String currentBarcode,
    required String currentQty,
  }) async {
    if (_scanning || _submitting) {
      return null;
    }

    _scanErrorMessage = null;
    _scanSuccessMessage = null;
    _pendingMissingBarcode = null;

    if (!canSubmit) {
      _scanErrorMessage = '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    final barcodeValue = barcode.trim();
    if (barcodeValue.isEmpty) {
      _scanErrorMessage = '请输入条码后再扫码入库';
      notifyListeners();
      return null;
    }

    _scanning = true;
    notifyListeners();

    try {
      final scanned = await _repository.scanProduct(barcodeValue);
      final defaultUnitCost =
          (scanned.lastInboundUnitCost?.trim().isNotEmpty ?? false)
              ? scanned.lastInboundUnitCost!.trim()
              : (scanned.costPrice?.trim() ?? '');
      final message = '扫码成功：#${scanned.id} ${scanned.name}，数量已记录为 1';
      _scanSuccessMessage = message;
      return InboundScanResult(
        productIdText: scanned.id.toString(),
        productName: scanned.name,
        barcodeText: barcodeValue,
        qtyText: '1',
        unitCostText: defaultUnitCost,
        message: message,
      );
    } catch (error) {
      if (error is ApiException && error.code == 4040) {
        _pendingMissingBarcode = barcodeValue;
      }
      _scanErrorMessage = _humanizeError(error, forScan: true);
      return null;
    } finally {
      _scanning = false;
      notifyListeners();
    }
  }

  Future<InboundScanPreview?> scanForConfirm({
    required String barcode,
  }) async {
    if (_scanning || _submitting) {
      return null;
    }

    _scanErrorMessage = null;
    _scanSuccessMessage = null;
    _pendingMissingBarcode = null;

    if (!canSubmit) {
      _scanErrorMessage = '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    final barcodeValue = barcode.trim();
    if (barcodeValue.isEmpty) {
      _scanErrorMessage = '请输入条码后再扫码入库';
      notifyListeners();
      return null;
    }

    _scanning = true;
    notifyListeners();

    try {
      final scanned = await _repository.scanProduct(barcodeValue);
      final defaultUnitCost =
          (scanned.lastInboundUnitCost?.trim().isNotEmpty ?? false)
              ? scanned.lastInboundUnitCost!.trim()
              : (scanned.costPrice?.trim() ?? '');
      _scanSuccessMessage = '扫码成功：#${scanned.id} ${scanned.name}，请确认后写入数量';
      return InboundScanPreview(
        productIdText: scanned.id.toString(),
        productName: scanned.name,
        barcodeText: barcodeValue,
        unitCostText: defaultUnitCost,
      );
    } catch (error) {
      if (error is ApiException && error.code == 4040) {
        _pendingMissingBarcode = barcodeValue;
      }
      _scanErrorMessage = _humanizeError(error, forScan: true);
      return null;
    } finally {
      _scanning = false;
      notifyListeners();
    }
  }

  InboundScanResult? applyScanConfirmed({
    required String productId,
    required String productName,
    required String barcode,
    required String qty,
    required String unitCost,
    required String currentProductId,
    required String currentBarcode,
    required String currentQty,
  }) {
    _scanErrorMessage = null;
    _scanSuccessMessage = null;

    if (!canSubmit) {
      _scanErrorMessage = '当前角色无采购入库权限，仅 OWNER/PURCHASER 可操作';
      notifyListeners();
      return null;
    }

    final appendQty = _parsePositiveInt(qty.trim());
    if (appendQty == null) {
      _scanErrorMessage = '确认区“数量”必须为正整数';
      notifyListeners();
      return null;
    }

    final message = '扫码成功：#${productId.trim()} $productName，数量已记录为 $appendQty';
    _scanSuccessMessage = message;
    notifyListeners();
    return InboundScanResult(
      productIdText: productId.trim(),
      productName: productName,
      barcodeText: barcode.trim(),
      qtyText: appendQty.toString(),
      unitCostText: unitCost.trim(),
      message: message,
    );
  }

  Future<bool> submit({
    required String productId,
    required String barcode,
    required String qty,
    required String unitCost,
    required String expectedVersion,
    required String remark,
  }) async {
    if (_submitting) {
      return false;
    }

    _errorMessage = null;
    _successMessage = null;

    final validationError = validateForm(
      productId: productId,
      barcode: barcode,
      qty: qty,
      unitCost: unitCost,
      expectedVersion: expectedVersion,
    );
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    final parsedProductId =
        productId.trim().isEmpty ? null : _parsePositiveInt(productId.trim());
    final parsedQty = _parsePositiveInt(qty.trim())!;
    final parsedExpectedVersion = expectedVersion.trim().isEmpty
        ? null
        : _parseNonNegativeInt(expectedVersion.trim());
    final barcodeValue = barcode.trim().isEmpty ? null : barcode.trim();
    final remarkValue = remark.trim().isEmpty ? null : remark.trim();
    final unitCostValue = unitCost.trim().isEmpty ? null : unitCost.trim();

    final request = InboundRequest(
      productId: parsedProductId,
      barcode: barcodeValue,
      qty: parsedQty,
      unitCost: unitCostValue,
      expectedVersion: parsedExpectedVersion,
      remark: remarkValue,
    );

    _submitting = true;
    notifyListeners();

    try {
      final response = await _repository.inbound(request);
      _result = response;
      _batchResult = null;
      _successMessage = '入库成功：${response.bizNo}，当前库存 ${response.currentStock}';
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  Future<bool> submitBatch({
    required List<InboundBatchFormItemInput> items,
  }) async {
    if (_submitting) {
      return false;
    }

    _errorMessage = null;
    _successMessage = null;

    final effectiveItems = items
        .where(
          (item) =>
              item.productId.trim().isNotEmpty ||
              item.barcode.trim().isNotEmpty ||
              item.qty.trim().isNotEmpty ||
              item.unitCost.trim().isNotEmpty ||
              item.expectedVersion.trim().isNotEmpty ||
              item.remark.trim().isNotEmpty,
        )
        .toList();

    if (effectiveItems.isEmpty) {
      _errorMessage = '请至少添加一条入库明细';
      notifyListeners();
      return false;
    }

    final batchRequests = <InboundBatchItemRequest>[];
    for (var i = 0; i < effectiveItems.length; i += 1) {
      final item = effectiveItems[i];
      final row = i + 1;

      if (item.productId.trim().isEmpty && item.barcode.trim().isEmpty) {
        _errorMessage = '第 $row 行需至少填写商品ID或条码';
        notifyListeners();
        return false;
      }

      final parsedProductId = item.productId.trim().isEmpty
          ? null
          : _parsePositiveInt(item.productId.trim());
      if (item.productId.trim().isNotEmpty && parsedProductId == null) {
        _errorMessage = '第 $row 行商品ID必须为正整数';
        notifyListeners();
        return false;
      }

      final parsedQty = _parsePositiveInt(item.qty.trim());
      if (parsedQty == null) {
        _errorMessage = '第 $row 行数量必须为正整数';
        notifyListeners();
        return false;
      }

      final unitCost = item.unitCost.trim();
      if (unitCost.isNotEmpty && !_isPriceValid(unitCost)) {
        _errorMessage = '第 $row 行进货价格式错误（示例：2.20）';
        notifyListeners();
        return false;
      }

      final parsedExpectedVersion = item.expectedVersion.trim().isEmpty
          ? null
          : _parseNonNegativeInt(item.expectedVersion.trim());
      if (item.expectedVersion.trim().isNotEmpty &&
          parsedExpectedVersion == null) {
        _errorMessage = '第 $row 行版本必须为大于等于 0 的整数';
        notifyListeners();
        return false;
      }

      batchRequests.add(
        InboundBatchItemRequest(
          productId: parsedProductId,
          barcode: item.barcode.trim().isEmpty ? null : item.barcode.trim(),
          qty: parsedQty,
          unitCost: unitCost.isEmpty ? null : unitCost,
          expectedVersion: parsedExpectedVersion,
          remark: item.remark.trim().isEmpty ? null : item.remark.trim(),
        ),
      );
    }

    _submitting = true;
    notifyListeners();

    try {
      final response = await _repository.inboundBatch(batchRequests);
      _batchResult = response;
      _result = null;
      _successMessage = '批量入库成功：${response.bizNo}，共 ${response.items.length} 条';
      return true;
    } catch (error) {
      _errorMessage = _humanizeError(error);
      return false;
    } finally {
      _submitting = false;
      notifyListeners();
    }
  }

  String _humanizeError(Object error, {bool forScan = false}) {
    if (error is ApiException) {
      if (forScan && error.code == 4040) {
        return '未找到该条码对应商品，可新建商品后重试。';
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

    return forScan ? '扫码入库失败，请稍后重试' : '入库失败，请稍后重试';
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

  bool _isPriceValid(String value) {
    return RegExp(r'^\d+(\.\d{1,4})?$').hasMatch(value);
  }
}
