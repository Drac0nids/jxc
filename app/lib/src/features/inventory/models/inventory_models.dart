class ScanProductData {
  const ScanProductData({
    required this.id,
    required this.sku,
    required this.barcode,
    required this.name,
    required this.unit,
    required this.currentStock,
    required this.retailPrice,
    required this.lastInboundUnitCost,
    required this.costPrice,
    required this.version,
    required this.minStockLimit,
  });

  final int id;
  final String sku;
  final String barcode;
  final String name;
  final String unit;
  final int currentStock;
  final String retailPrice;
  final String? lastInboundUnitCost;
  final String? costPrice;
  final int version;
  final int minStockLimit;

  factory ScanProductData.fromJson(Map<String, dynamic> json) {
    return ScanProductData(
      id: _toInt(json['id']),
      sku: (json['sku'] ?? '').toString(),
      barcode: (json['barcode'] ?? '').toString(),
      name: (json['name'] ?? '').toString(),
      unit: (json['unit'] ?? '').toString(),
      currentStock: _toInt(json['current_stock']),
      retailPrice: (json['retail_price'] ?? '0').toString(),
      lastInboundUnitCost: json['last_inbound_unit_cost']?.toString(),
      costPrice: json['cost_price']?.toString(),
      version: _toInt(json['version']),
      minStockLimit: _toInt(json['min_stock_limit']),
    );
  }
}

class InboundRequest {
  const InboundRequest({
    this.productId,
    this.barcode,
    required this.qty,
    this.unitCost,
    this.expectedVersion,
    this.bizNo,
    this.remark,
  });

  final int? productId;
  final String? barcode;
  final int qty;
  final String? unitCost;
  final int? expectedVersion;
  final String? bizNo;
  final String? remark;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (productId != null) 'product_id': productId,
      if (barcode != null) 'barcode': barcode,
      'qty': qty,
      if (unitCost != null && unitCost!.trim().isNotEmpty)
        'unit_cost': unitCost,
      if (expectedVersion != null) 'expected_version': expectedVersion,
      if (bizNo != null) 'biz_no': bizNo,
      if (remark != null) 'remark': remark,
    };
  }
}

class InboundResultData {
  const InboundResultData({
    required this.bizNo,
    required this.productId,
    required this.currentStock,
    required this.costPrice,
    required this.version,
  });

  final String bizNo;
  final int productId;
  final int currentStock;
  final String costPrice;
  final int version;

  factory InboundResultData.fromJson(Map<String, dynamic> json) {
    return InboundResultData(
      bizNo: (json['biz_no'] ?? '').toString(),
      productId: _toInt(json['product_id']),
      currentStock: _toInt(json['current_stock']),
      costPrice: (json['cost_price'] ?? '0').toString(),
      version: _toInt(json['version']),
    );
  }
}

class InboundBatchItemRequest {
  const InboundBatchItemRequest({
    this.productId,
    this.barcode,
    required this.qty,
    this.unitCost,
    this.expectedVersion,
    this.remark,
  });

  final int? productId;
  final String? barcode;
  final int qty;
  final String? unitCost;
  final int? expectedVersion;
  final String? remark;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (productId != null) 'product_id': productId,
      if (barcode != null && barcode!.trim().isNotEmpty) 'barcode': barcode,
      'qty': qty,
      if (unitCost != null && unitCost!.trim().isNotEmpty)
        'unit_cost': unitCost,
      if (expectedVersion != null) 'expected_version': expectedVersion,
      if (remark != null && remark!.trim().isNotEmpty) 'remark': remark,
    };
  }
}

class InboundBatchItemResultData {
  const InboundBatchItemResultData({
    required this.productId,
    required this.currentStock,
    required this.costPrice,
    required this.version,
  });

  final int productId;
  final int currentStock;
  final String costPrice;
  final int version;

  factory InboundBatchItemResultData.fromJson(Map<String, dynamic> json) {
    return InboundBatchItemResultData(
      productId: _toInt(json['product_id']),
      currentStock: _toInt(json['current_stock']),
      costPrice: (json['cost_price'] ?? '0').toString(),
      version: _toInt(json['version']),
    );
  }
}

class InboundBatchResultData {
  const InboundBatchResultData({
    required this.bizNo,
    required this.items,
  });

  final String bizNo;
  final List<InboundBatchItemResultData> items;

  factory InboundBatchResultData.fromJson(Map<String, dynamic> json) {
    final rawItems = json['items'];
    final items = (rawItems is List)
        ? rawItems
            .whereType<Map<String, dynamic>>()
            .map(InboundBatchItemResultData.fromJson)
            .toList()
        : <InboundBatchItemResultData>[];

    return InboundBatchResultData(
      bizNo: (json['biz_no'] ?? '').toString(),
      items: items,
    );
  }
}

class OutboundItemRequest {
  const OutboundItemRequest({
    required this.productId,
    required this.qty,
    required this.sellPrice,
    this.expectedVersion,
  });

  final int productId;
  final int qty;
  final String sellPrice;
  final int? expectedVersion;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      'product_id': productId,
      'qty': qty,
      'sell_price': sellPrice,
      if (expectedVersion != null) 'expected_version': expectedVersion,
    };
  }
}

class OutboundRequest {
  const OutboundRequest({
    this.bizNo,
    this.customerId,
    this.expectedVersion,
    required this.items,
    this.remark,
  });

  final String? bizNo;
  final int? customerId;
  final int? expectedVersion;
  final List<OutboundItemRequest> items;
  final String? remark;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (bizNo != null) 'biz_no': bizNo,
      if (customerId != null) 'customer_id': customerId,
      if (expectedVersion != null) 'expected_version': expectedVersion,
      'items': items.map((OutboundItemRequest item) => item.toJson()).toList(),
      if (remark != null) 'remark': remark,
    };
  }
}

class OutboundItemResult {
  const OutboundItemResult({
    required this.productId,
    required this.qty,
    required this.currentStock,
    required this.version,
  });

  final int productId;
  final int qty;
  final int currentStock;
  final int version;

  factory OutboundItemResult.fromJson(Map<String, dynamic> json) {
    return OutboundItemResult(
      productId: _toInt(json['product_id']),
      qty: _toInt(json['qty']),
      currentStock: _toInt(json['current_stock']),
      version: _toInt(json['version']),
    );
  }
}

class OutboundResultData {
  const OutboundResultData({
    required this.bizNo,
    required this.items,
    required this.totalAmount,
  });

  final String bizNo;
  final List<OutboundItemResult> items;
  final String totalAmount;

  factory OutboundResultData.fromJson(Map<String, dynamic> json) {
    final rawItems = json['items'];
    final items = (rawItems is List)
        ? rawItems
            .whereType<Map<String, dynamic>>()
            .map(OutboundItemResult.fromJson)
            .toList()
        : <OutboundItemResult>[];

    return OutboundResultData(
      bizNo: (json['biz_no'] ?? '').toString(),
      items: items,
      totalAmount: (json['total_amount'] ?? '0').toString(),
    );
  }
}

class StockCheckCreateItemRequest {
  const StockCheckCreateItemRequest({required this.productId});

  final int productId;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      'product_id': productId,
    };
  }
}

class StockCheckCreateRequest {
  const StockCheckCreateRequest({
    this.bizNo,
    required this.items,
    this.remark,
  });

  final String? bizNo;
  final List<StockCheckCreateItemRequest> items;
  final String? remark;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (bizNo != null) 'biz_no': bizNo,
      'items': items
          .map((StockCheckCreateItemRequest item) => item.toJson())
          .toList(),
      if (remark != null) 'remark': remark,
    };
  }
}

class StockCheckConfirmItemRequest {
  const StockCheckConfirmItemRequest({
    required this.productId,
    required this.actualStock,
  });

  final int productId;
  final int actualStock;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      'product_id': productId,
      'actual_stock': actualStock,
    };
  }
}

class StockCheckConfirmRequest {
  const StockCheckConfirmRequest({
    this.expectedVersion,
    required this.items,
    this.remark,
  });

  final int? expectedVersion;
  final List<StockCheckConfirmItemRequest> items;
  final String? remark;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (expectedVersion != null) 'expected_version': expectedVersion,
      'items': items
          .map((StockCheckConfirmItemRequest item) => item.toJson())
          .toList(),
      if (remark != null) 'remark': remark,
    };
  }
}

class StockCheckItemData {
  const StockCheckItemData({
    required this.productId,
    required this.bookStock,
    this.actualStock,
    this.deltaQty,
  });

  final int productId;
  final int bookStock;
  final int? actualStock;
  final int? deltaQty;

  factory StockCheckItemData.fromJson(Map<String, dynamic> json) {
    return StockCheckItemData(
      productId: _toInt(json['product_id']),
      bookStock: _toInt(json['book_stock']),
      actualStock: _toNullableInt(json['actual_stock']),
      deltaQty: _toNullableInt(json['delta_qty']),
    );
  }
}

class StockCheckData {
  const StockCheckData({
    required this.id,
    required this.bizNo,
    required this.status,
    required this.items,
    required this.remark,
    required this.version,
    required this.countingAt,
    required this.confirmedAt,
    required this.createdAt,
    required this.updatedAt,
  });

  final int id;
  final String bizNo;
  final String status;
  final List<StockCheckItemData> items;
  final String? remark;
  final int version;
  final String? countingAt;
  final String? confirmedAt;
  final String createdAt;
  final String updatedAt;

  factory StockCheckData.fromJson(Map<String, dynamic> json) {
    final rawItems = json['items'];
    final items = (rawItems is List)
        ? rawItems
            .whereType<Map<String, dynamic>>()
            .map(StockCheckItemData.fromJson)
            .toList()
        : <StockCheckItemData>[];

    return StockCheckData(
      id: _toInt(json['id']),
      bizNo: (json['biz_no'] ?? '').toString(),
      status: (json['status'] ?? '').toString(),
      items: items,
      remark: json['remark']?.toString(),
      version: _toInt(json['version']),
      countingAt: json['counting_at']?.toString(),
      confirmedAt: json['confirmed_at']?.toString(),
      createdAt: (json['created_at'] ?? '').toString(),
      updatedAt: (json['updated_at'] ?? '').toString(),
    );
  }
}

class PurchaseOrderItemData {
  const PurchaseOrderItemData({
    required this.productId,
    required this.productName,
    required this.qty,
    required this.unitCost,
    required this.lineAmount,
  });

  final int productId;
  final String productName;
  final int qty;
  final String unitCost;
  final String lineAmount;

  factory PurchaseOrderItemData.fromJson(Map<String, dynamic> json) {
    return PurchaseOrderItemData(
      productId: _toInt(json['product_id']),
      productName: (json['product_name'] ?? '').toString(),
      qty: _toInt(json['qty']),
      unitCost: (json['unit_cost'] ?? '0').toString(),
      lineAmount: (json['line_amount'] ?? '0').toString(),
    );
  }
}

class PurchaseOrderData {
  const PurchaseOrderData({
    required this.id,
    required this.bizNo,
    required this.supplierId,
    required this.status,
    required this.items,
    required this.totalAmount,
    required this.remark,
    required this.version,
    required this.confirmedAt,
    required this.voidedAt,
    required this.createdAt,
    required this.updatedAt,
  });

  final int id;
  final String bizNo;
  final int? supplierId;
  final String status;
  final List<PurchaseOrderItemData> items;
  final String totalAmount;
  final String? remark;
  final int version;
  final String? confirmedAt;
  final String? voidedAt;
  final String createdAt;
  final String updatedAt;

  factory PurchaseOrderData.fromJson(Map<String, dynamic> json) {
    final rawItems = json['items'];
    final items = (rawItems is List)
        ? rawItems
            .whereType<Map<String, dynamic>>()
            .map(PurchaseOrderItemData.fromJson)
            .toList()
        : <PurchaseOrderItemData>[];

    return PurchaseOrderData(
      id: _toInt(json['id']),
      bizNo: (json['biz_no'] ?? '').toString(),
      supplierId: _toNullableInt(json['supplier_id']),
      status: (json['status'] ?? '').toString(),
      items: items,
      totalAmount: (json['total_amount'] ?? '0').toString(),
      remark: json['remark']?.toString(),
      version: _toInt(json['version']),
      confirmedAt: json['confirmed_at']?.toString(),
      voidedAt: json['voided_at']?.toString(),
      createdAt: (json['created_at'] ?? '').toString(),
      updatedAt: (json['updated_at'] ?? '').toString(),
    );
  }
}

class PurchaseOrdersPageData {
  const PurchaseOrdersPageData({
    required this.startDate,
    required this.endDate,
    required this.list,
    required this.total,
    required this.page,
    required this.pageSize,
  });

  final String startDate;
  final String endDate;
  final List<PurchaseOrderData> list;
  final int total;
  final int page;
  final int pageSize;

  factory PurchaseOrdersPageData.fromJson(Map<String, dynamic> json) {
    final rawList = json['list'];
    final list = (rawList is List)
        ? rawList
            .whereType<Map<String, dynamic>>()
            .map(PurchaseOrderData.fromJson)
            .toList()
        : <PurchaseOrderData>[];

    return PurchaseOrdersPageData(
      startDate: (json['start_date'] ?? '').toString(),
      endDate: (json['end_date'] ?? '').toString(),
      list: list,
      total: _toInt(json['total']),
      page: _toInt(json['page']),
      pageSize: _toInt(json['page_size']),
    );
  }
}

class StockLogData {
  const StockLogData({
    required this.id,
    required this.productId,
    required this.productName,
    required this.bizType,
    required this.bizNo,
    required this.deltaQty,
    required this.snapshotStock,
    required this.snapshotCost,
    required this.snapshotSellPrice,
    required this.snapshotInboundUnitCost,
    required this.operatorId,
    required this.operatorName,
    required this.createdAt,
  });

  final int id;
  final int productId;
  final String? productName;
  final String bizType;
  final String bizNo;
  final int deltaQty;
  final int snapshotStock;
  final String snapshotCost;
  final String? snapshotSellPrice;
  final String? snapshotInboundUnitCost;
  final String operatorId;
  final String? operatorName;
  final String createdAt;

  factory StockLogData.fromJson(Map<String, dynamic> json) {
    return StockLogData(
      id: _toInt(json['id']),
      productId: _toInt(json['product_id']),
      productName: json['product_name']?.toString(),
      bizType: (json['biz_type'] ?? '').toString(),
      bizNo: (json['biz_no'] ?? '').toString(),
      deltaQty: _toInt(json['delta_qty']),
      snapshotStock: _toInt(json['snapshot_stock']),
      snapshotCost: (json['snapshot_cost'] ?? '0').toString(),
      snapshotSellPrice: json['snapshot_sell_price']?.toString(),
      snapshotInboundUnitCost: json['snapshot_inbound_unit_cost']?.toString(),
      operatorId: (json['operator_id'] ?? '').toString(),
      operatorName: json['operator_name']?.toString(),
      createdAt: (json['created_at'] ?? '').toString(),
    );
  }
}

class InboundLogsPageData {
  const InboundLogsPageData({
    required this.list,
    required this.total,
    required this.page,
    required this.pageSize,
  });

  final List<StockLogData> list;
  final int total;
  final int page;
  final int pageSize;

  factory InboundLogsPageData.fromJson(Map<String, dynamic> json) {
    final rawList = json['list'];
    final list = (rawList is List)
        ? rawList
            .whereType<Map<String, dynamic>>()
            .map(StockLogData.fromJson)
            .toList()
        : <StockLogData>[];

    return InboundLogsPageData(
      list: list,
      total: _toInt(json['total']),
      page: _toInt(json['page']),
      pageSize: _toInt(json['page_size']),
    );
  }
}

int _toInt(Object? value) {
  if (value is int) {
    return value;
  }
  if (value is num) {
    return value.toInt();
  }
  if (value is String) {
    return int.tryParse(value) ?? 0;
  }

  return 0;
}

int? _toNullableInt(Object? value) {
  if (value == null) {
    return null;
  }

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
