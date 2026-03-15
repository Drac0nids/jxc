class ProductData {
  const ProductData({
    required this.id,
    required this.sku,
    required this.barcode,
    required this.name,
    required this.unit,
    required this.currentStock,
    required this.retailPrice,
    required this.lastInboundUnitCost,
    required this.costPrice,
    required this.minStockLimit,
    required this.version,
    this.categoryId,
    this.trackBatches = false,
    this.trackSerials = false,
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
  final int minStockLimit;
  final int version;
  final int? categoryId;
  final bool trackBatches;
  final bool trackSerials;

  factory ProductData.fromJson(Map<String, dynamic> json) {
    return ProductData(
      id: _toInt(json['id']),
      sku: (json['sku'] ?? '').toString(),
      barcode: (json['barcode'] ?? '').toString(),
      name: (json['name'] ?? '').toString(),
      unit: (json['unit'] ?? '').toString(),
      currentStock: _toInt(json['current_stock']),
      retailPrice: (json['retail_price'] ?? '0').toString(),
      lastInboundUnitCost: json['last_inbound_unit_cost']?.toString(),
      costPrice: json['cost_price']?.toString(),
      minStockLimit: _toInt(json['min_stock_limit']),
      version: _toInt(json['version']),
      categoryId: json['category_id'] is int ? json['category_id'] as int : null,
      trackBatches: json['track_batches'] == true,
      trackSerials: json['track_serials'] == true,
    );
  }
}

class PagedData<T> {
  const PagedData({
    required this.list,
    required this.total,
    required this.page,
    required this.pageSize,
  });

  final List<T> list;
  final int total;
  final int page;
  final int pageSize;
}

class ListProductsQuery {
  const ListProductsQuery({
    this.page,
    this.pageSize,
    this.keyword,
    this.barcode,
    this.lowStock,
    this.categoryId,
  });

  final int? page;
  final int? pageSize;
  final String? keyword;
  final String? barcode;
  final bool? lowStock;
  final int? categoryId;

  Map<String, dynamic> toQuery() {
    return <String, dynamic>{
      if (page != null) 'page': page,
      if (pageSize != null) 'page_size': pageSize,
      if (keyword != null && keyword!.trim().isNotEmpty)
        'keyword': keyword!.trim(),
      if (barcode != null && barcode!.trim().isNotEmpty)
        'barcode': barcode!.trim(),
      if (lowStock == true) 'low_stock': 'true',
      if (categoryId != null) 'category_id': categoryId,
    };
  }
}

class CreateProductRequest {
  const CreateProductRequest({
    this.sku,
    required this.barcode,
    required this.name,
    required this.unit,
    required this.retailPrice,
    this.initStock,
    this.minStockLimit,
    required this.costPrice,
    this.categoryId,
    this.trackBatches,
    this.trackSerials,
  });

  final String? sku;
  final String barcode;
  final String name;
  final String unit;
  final String retailPrice;
  final int? initStock;
  final int? minStockLimit;
  final String costPrice;
  final int? categoryId;
  final bool? trackBatches;
  final bool? trackSerials;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (sku != null && sku!.trim().isNotEmpty) 'sku': sku!.trim(),
      'barcode': barcode,
      'name': name,
      'unit': unit,
      'retail_price': retailPrice,
      if (initStock != null) 'init_stock': initStock,
      if (minStockLimit != null) 'min_stock_limit': minStockLimit,
      'cost_price': costPrice.trim(),
      if (categoryId != null) 'category_id': categoryId,
      if (trackBatches != null) 'track_batches': trackBatches,
      if (trackSerials != null) 'track_serials': trackSerials,
    };
  }
}

class UpdateProductRequest {
  const UpdateProductRequest({
    this.sku,
    this.barcode,
    this.name,
    this.unit,
    this.retailPrice,
    this.minStockLimit,
    this.expectedVersion,
    this.categoryId,
    this.trackBatches,
    this.trackSerials,
  });

  final String? sku;
  final String? barcode;
  final String? name;
  final String? unit;
  final String? retailPrice;
  final int? minStockLimit;
  final int? expectedVersion;
  final int? categoryId;
  final bool? trackBatches;
  final bool? trackSerials;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      if (sku != null && sku!.trim().isNotEmpty) 'sku': sku!.trim(),
      if (barcode != null && barcode!.trim().isNotEmpty)
        'barcode': barcode!.trim(),
      if (name != null && name!.trim().isNotEmpty) 'name': name!.trim(),
      if (unit != null && unit!.trim().isNotEmpty) 'unit': unit!.trim(),
      if (retailPrice != null && retailPrice!.trim().isNotEmpty)
        'retail_price': retailPrice!.trim(),
      if (minStockLimit != null) 'min_stock_limit': minStockLimit,
      if (expectedVersion != null) 'expected_version': expectedVersion,
      if (categoryId != null) 'category_id': categoryId,
      if (trackBatches != null) 'track_batches': trackBatches,
      if (trackSerials != null) 'track_serials': trackSerials,
    };
  }
}

class DeleteProductResult {
  const DeleteProductResult({
    required this.id,
    required this.deleted,
    required this.version,
  });

  final int id;
  final bool deleted;
  final int version;

  factory DeleteProductResult.fromJson(Map<String, dynamic> json) {
    return DeleteProductResult(
      id: _toInt(json['id']),
      deleted: json['deleted'] == true,
      version: _toInt(json['version']),
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

// ── ProductData 便利扩展 ───────────────────────────────────────────────────

extension ProductDataX on ProductData {
  /// 毛利率（按均摊成本）：`(retail - cost) / retail`，返回 0~100 的百分比字符串
  /// 若数据缺失则返回 null
  String? get grossMarginStr {
    final double? retail = double.tryParse(retailPrice.replaceAll('¥', ''));
    final double? cost = costPrice != null
        ? double.tryParse(costPrice!.replaceAll('¥', ''))
        : null;
    if (retail == null || cost == null || retail <= 0) return null;
    final double margin = (retail - cost) / retail * 100;
    return '${margin.toStringAsFixed(1)}%';
  }

  /// 是否低库存
  bool get isLowStock => minStockLimit > 0 && currentStock < minStockLimit;

  /// 缺货数量（低库存时有值）
  int get shortage => isLowStock ? minStockLimit - currentStock : 0;

  /// 库存进度（0.0 ~ 1.0），用于进度条
  double get stockProgress {
    if (minStockLimit > 0) {
      return (currentStock / minStockLimit).clamp(0.0, 1.0);
    }
    if (currentStock <= 0) return 0.0;
    return (currentStock / (currentStock + 20)).clamp(0.0, 1.0);
  }
}
