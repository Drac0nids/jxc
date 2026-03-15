import 'package:flutter/foundation.dart';

import '../../../core/models/user_role.dart';
import '../../../network/api_exception.dart';
import '../models/inventory_models.dart';
import '../models/inventory_repository.dart';

class OutboundFormItemInput {
  const OutboundFormItemInput({
    required this.productId,
    required this.productName,
    required this.qty,
    required this.sellPrice,
    required this.expectedVersion,
  });

  final String productId;
  final String productName;
  final String qty;
  final String sellPrice;
  final String expectedVersion;

  OutboundFormItemInput copyWith({
    String? productId,
    String? productName,
    String? qty,
    String? sellPrice,
    String? expectedVersion,
  }) {
    return OutboundFormItemInput(
      productId: productId ?? this.productId,
      productName: productName ?? this.productName,
      qty: qty ?? this.qty,
      sellPrice: sellPrice ?? this.sellPrice,
      expectedVersion: expectedVersion ?? this.expectedVersion,
    );
  }
}

class OutboundScanResult {
  const OutboundScanResult({
    required this.items,
    required this.message,
  });

  final List<OutboundFormItemInput> items;
  final String message;
}

class OutboundScanPreview {
  const OutboundScanPreview({
    required this.productId,
    required this.productName,
    required this.suggestedSellPrice,
    required this.currentStock,
  });

  final String productId;
  final String productName;
  final String suggestedSellPrice;
  final int currentStock;
}

class OutboundController extends ChangeNotifier {
  OutboundController({
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
  OutboundResultData? _result;

  bool get canSubmit => _role == UserRole.owner || _role == UserRole.sales;
  bool get submitting => _submitting;
  bool get scanning => _scanning;
  String? get errorMessage => _errorMessage;
  String? get successMessage => _successMessage;
  String? get scanErrorMessage => _scanErrorMessage;
  String? get scanSuccessMessage => _scanSuccessMessage;
  OutboundResultData? get result => _result;

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

  void clearResult() {
    _result = null;
    notifyListeners();
  }

  String? validateForm({
    required String customerId,
    required String expectedVersion,
    required List<OutboundFormItemInput> items,
  }) {
    if (!canSubmit) {
      return '当前角色无销售出库权限，仅 OWNER/SALES 可操作';
    }

    if (customerId.trim().isNotEmpty && _parsePositiveInt(customerId) == null) {
      return '客户ID必须为正整数';
    }

    if (expectedVersion.trim().isNotEmpty &&
        _parseNonNegativeInt(expectedVersion) == null) {
      return '单据版本必须为大于等于 0 的整数';
    }

    final effectiveItems = _filterEffectiveItems(items);
    if (effectiveItems.isEmpty) {
      return '请至少添加一条出库明细';
    }

    for (int index = 0; index < effectiveItems.length; index += 1) {
      final item = effectiveItems[index];
      final row = index + 1;

      if (_parsePositiveInt(item.productId) == null) {
        return '第 $row 行商品ID必须为正整数';
      }
      if (_parsePositiveInt(item.qty) == null) {
        return '第 $row 行数量必须为正整数';
      }
      if (!_isPriceValid(item.sellPrice.trim())) {
        return '第 $row 行销售单价格式错误（示例：3.50）';
      }
      if (item.expectedVersion.trim().isNotEmpty &&
          _parseNonNegativeInt(item.expectedVersion) == null) {
        return '第 $row 行版本必须为大于等于 0 的整数';
      }
    }

    return null;
  }

  Future<OutboundScanResult?> scanAndAccumulate({
    required String barcode,
    required String customSellPrice,
    required List<OutboundFormItemInput> items,
  }) async {
    if (_scanning || _submitting) {
      return null;
    }

    _scanErrorMessage = null;
    _scanSuccessMessage = null;

    if (!canSubmit) {
      _scanErrorMessage = '当前角色无销售出库权限，仅 OWNER/SALES 可操作';
      notifyListeners();
      return null;
    }

    final barcodeValue = barcode.trim();
    if (barcodeValue.isEmpty) {
      _scanErrorMessage = '请输入条码后再扫码出库';
      notifyListeners();
      return null;
    }

    final customPriceValue = customSellPrice.trim();
    if (customPriceValue.isNotEmpty && !_isPriceValid(customPriceValue)) {
      _scanErrorMessage = '扫码区“销售单价”格式错误（示例：3.50）';
      notifyListeners();
      return null;
    }

    _scanning = true;
    notifyListeners();

    try {
      final scanned = await _repository.scanProduct(barcodeValue);
      final scannedProductIdText = scanned.id.toString();
      final sellPrice =
          customPriceValue.isEmpty ? scanned.retailPrice : customPriceValue;

      final nextItems =
          _filterEffectiveItems(items).map((item) => item.copyWith()).toList();
      final existedIndex = nextItems
          .indexWhere((item) => item.productId.trim() == scannedProductIdText);

      if (existedIndex >= 0) {
        final existed = nextItems[existedIndex];
        final currentQty = int.tryParse(existed.qty.trim());
        final safeQty = (currentQty != null && currentQty > 0) ? currentQty : 0;
        final nextQty = safeQty + 1;

        nextItems[existedIndex] = existed.copyWith(
          productName: existed.productName.trim().isEmpty
              ? scanned.name
              : existed.productName,
          qty: nextQty.toString(),
          sellPrice:
              existed.sellPrice.trim().isEmpty ? sellPrice : existed.sellPrice,
        );

        final message =
            '扫码成功：#${scanned.id} ${scanned.name}，已累加到第 ${existedIndex + 1} 个匹配明细，数量=$nextQty';
        _scanSuccessMessage = message;
        return OutboundScanResult(items: nextItems, message: message);
      }

      nextItems.add(
        OutboundFormItemInput(
          productId: scannedProductIdText,
          productName: scanned.name,
          qty: '1',
          sellPrice: sellPrice,
          expectedVersion: '',
        ),
      );

      final message = '扫码成功：#${scanned.id} ${scanned.name}，已新增出库明细（数量=1）';
      _scanSuccessMessage = message;
      return OutboundScanResult(items: nextItems, message: message);
    } catch (error) {
      _scanErrorMessage = _humanizeError(error, forScan: true);
      return null;
    } finally {
      _scanning = false;
      notifyListeners();
    }
  }

  Future<OutboundScanPreview?> scanForConfirm({
    required String barcode,
    required String customSellPrice,
  }) async {
    if (_scanning || _submitting) {
      return null;
    }

    _scanErrorMessage = null;
    _scanSuccessMessage = null;

    if (!canSubmit) {
      _scanErrorMessage = '当前角色无销售出库权限，仅 OWNER/SALES 可操作';
      notifyListeners();
      return null;
    }

    final barcodeValue = barcode.trim();
    if (barcodeValue.isEmpty) {
      _scanErrorMessage = '请输入条码后再扫码出库';
      notifyListeners();
      return null;
    }

    final customPriceValue = customSellPrice.trim();
    if (customPriceValue.isNotEmpty && !_isPriceValid(customPriceValue)) {
      _scanErrorMessage = '扫码区“销售单价”格式错误（示例：3.50）';
      notifyListeners();
      return null;
    }

    _scanning = true;
    notifyListeners();

    try {
      final scanned = await _repository.scanProduct(barcodeValue);
      final sellPrice =
          customPriceValue.isEmpty ? scanned.retailPrice : customPriceValue;
      _scanSuccessMessage = '扫码成功：#${scanned.id} ${scanned.name}，请确认后写入明细';
      return OutboundScanPreview(
        productId: scanned.id.toString(),
        productName: scanned.name,
        suggestedSellPrice: sellPrice,
        currentStock: scanned.currentStock,
      );
    } catch (error) {
      _scanErrorMessage = _humanizeError(error, forScan: true);
      return null;
    } finally {
      _scanning = false;
      notifyListeners();
    }
  }

  Future<bool> submit({
    required String customerId,
    required String expectedVersion,
    required String remark,
    required List<OutboundFormItemInput> items,
  }) async {
    if (_submitting) {
      return false;
    }

    _errorMessage = null;
    _successMessage = null;

    final validationError = validateForm(
      customerId: customerId,
      expectedVersion: expectedVersion,
      items: items,
    );
    if (validationError != null) {
      _errorMessage = validationError;
      notifyListeners();
      return false;
    }

    final parsedCustomerId =
        customerId.trim().isEmpty ? null : _parsePositiveInt(customerId.trim());
    final parsedExpectedVersion = expectedVersion.trim().isEmpty
        ? null
        : _parseNonNegativeInt(expectedVersion.trim());
    final remarkValue = remark.trim().isEmpty ? null : remark.trim();

    final requestItems = _filterEffectiveItems(items).map((item) {
      return OutboundItemRequest(
        productId: _parsePositiveInt(item.productId.trim())!,
        qty: _parsePositiveInt(item.qty.trim())!,
        sellPrice: item.sellPrice.trim(),
        expectedVersion: item.expectedVersion.trim().isEmpty
            ? null
            : _parseNonNegativeInt(item.expectedVersion.trim()),
      );
    }).toList();

    final request = OutboundRequest(
      customerId: parsedCustomerId,
      expectedVersion: parsedExpectedVersion,
      items: requestItems,
      remark: remarkValue,
    );

    _submitting = true;
    notifyListeners();

    try {
      final response = await _repository.outbound(request);
      _result = response;
      _successMessage = '出库成功：${response.bizNo}，总金额 ${response.totalAmount}';
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
        return '未找到该条码对应商品，请先前往“商品管理”建档。';
      }

      if (error.code == 4001) {
        final detail = _parseStockInsufficient(error.data);
        if (detail != null) {
          return '库存不足：商品ID=${detail.failedProductId}，可用库存=${detail.availableStock}，需求数量=${detail.requiredQty}${_requestIdPart(error.requestId)}';
        }
      }

      if (error.code == 4091) {
        return '版本冲突，请刷新后重试（code=4091）';
      }

      return '${error.message}（code=${error.code}${_requestIdPart(error.requestId)}）';
    }

    if (error is FormatException) {
      return error.message;
    }

    if (error is Exception) {
      return error.toString().replaceFirst('Exception: ', '');
    }

    return forScan ? '扫码出库失败，请稍后重试' : '出库失败，请稍后重试';
  }

  String _requestIdPart(String? requestId) {
    if (requestId == null || requestId.isEmpty) {
      return '';
    }

    return '，request_id=$requestId';
  }

  _StockInsufficientDetail? _parseStockInsufficient(Object? raw) {
    if (raw is! Map<String, dynamic>) {
      return null;
    }

    final failedProductId = _toInt(raw['failed_product_id']);
    final availableStock = _toInt(raw['available_stock']);
    final requiredQty = _toInt(raw['required_qty']);
    if (failedProductId == null ||
        availableStock == null ||
        requiredQty == null) {
      return null;
    }

    return _StockInsufficientDetail(
      failedProductId: failedProductId,
      availableStock: availableStock,
      requiredQty: requiredQty,
    );
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

  int? _toInt(Object? value) {
    if (value is int) {
      return value;
    }
    if (value is num) {
      return value.toInt();
    }
    if (value is String) {
      return int.tryParse(value);
    }

    return null;
  }

  bool _isPriceValid(String value) {
    return RegExp(r'^\d+(\.\d{1,4})?$').hasMatch(value);
  }

  List<OutboundFormItemInput> _filterEffectiveItems(
      List<OutboundFormItemInput> items) {
    return items
        .where(
          (item) =>
              item.productId.trim().isNotEmpty ||
              item.productName.trim().isNotEmpty ||
              item.qty.trim().isNotEmpty ||
              item.sellPrice.trim().isNotEmpty ||
              item.expectedVersion.trim().isNotEmpty,
        )
        .toList();
  }
}

class _StockInsufficientDetail {
  const _StockInsufficientDetail({
    required this.failedProductId,
    required this.availableStock,
    required this.requiredQty,
  });

  final int failedProductId;
  final int availableStock;
  final int requiredQty;
}
