import 'dart:convert';

class DashboardData {
  const DashboardData({
    required this.date,
    required this.totalSales,
    required this.totalGrossProfit,
    required this.totalOrders,
    required this.lowStockCount,
    required this.topSellingItem,
  });

  final String date;
  final String totalSales;
  final String totalGrossProfit;
  final int totalOrders;
  final int lowStockCount;
  final String topSellingItem;

  factory DashboardData.fromJson(Map<String, dynamic> json) {
    return DashboardData(
      date: (json['date'] ?? '').toString(),
      totalSales: (json['total_sales'] ?? '0').toString(),
      totalGrossProfit: (json['total_gross_profit'] ?? '0').toString(),
      totalOrders: toInt(json['total_orders']),
      lowStockCount: toInt(json['low_stock_count']),
      topSellingItem: (json['top_selling_item'] ?? '-').toString(),
    );
  }
}

class DashboardOrdersDrilldownData {
  const DashboardOrdersDrilldownData({
    required this.startDate,
    required this.endDate,
    required this.list,
    required this.total,
    required this.page,
    required this.pageSize,
  });

  final String startDate;
  final String endDate;
  final List<SalesOrderData> list;
  final int total;
  final int page;
  final int pageSize;

  factory DashboardOrdersDrilldownData.fromJson(Map<String, dynamic> json) {
    final List<SalesOrderData> parsedList = toSalesOrderList(json);
    final int parsedTotal = toInt(json['total']);

    return DashboardOrdersDrilldownData(
      startDate: (json['start_date'] ?? '').toString(),
      endDate: (json['end_date'] ?? '').toString(),
      list: parsedList,
      total: parsedTotal > 0 ? parsedTotal : parsedList.length,
      page: toInt(json['page']),
      pageSize: toInt(json['page_size']),
    );
  }
}

class SalesOrderData {
  const SalesOrderData({
    required this.id,
    required this.bizNo,
    required this.customerId,
    required this.status,
    required this.items,
    required this.totalAmount,
    required this.remark,
    required this.version,
    required this.confirmedAt,
    required this.returnedAt,
    required this.voidedAt,
    required this.createdAt,
    required this.updatedAt,
  });

  final int id;
  final String bizNo;
  final int? customerId;
  final String status;
  final List<SalesOrderItemData> items;
  final String totalAmount;
  final String? remark;
  final int version;
  final String? confirmedAt;
  final String? returnedAt;
  final String? voidedAt;
  final String createdAt;
  final String updatedAt;

  factory SalesOrderData.fromJson(Map<String, dynamic> json) {
    final List<SalesOrderItemData> parsedItems = toSalesOrderItemList(
      json['items'] ?? json['details'] ?? json['list'],
    );

    return SalesOrderData(
      id: toInt(json['id']),
      bizNo: (json['biz_no'] ?? '').toString(),
      customerId: toNullableInt(json['customer_id']),
      status: (json['status'] ?? '').toString(),
      items: parsedItems,
      totalAmount: (json['total_amount'] ?? '0').toString(),
      remark: json['remark']?.toString(),
      version: toInt(json['version']),
      confirmedAt: json['confirmed_at']?.toString(),
      returnedAt: json['returned_at']?.toString(),
      voidedAt: json['voided_at']?.toString(),
      createdAt: (json['created_at'] ?? '').toString(),
      updatedAt: (json['updated_at'] ?? '').toString(),
    );
  }
}

class SalesOrderItemData {
  const SalesOrderItemData({
    required this.productId,
    required this.productName,
    required this.qty,
    required this.sellPrice,
    required this.lineAmount,
    required this.returnedQty,
  });

  final int productId;
  final String productName;
  final int qty;
  final String sellPrice;
  final String lineAmount;
  final int returnedQty;

  factory SalesOrderItemData.fromJson(Map<String, dynamic> json) {
    final int productId = toInt(json['product_id']);
    final String rawName = (json['product_name'] ?? '').toString().trim();
    final String safeProductName = rawName.isEmpty ? '商品#$productId' : rawName;

    return SalesOrderItemData(
      productId: productId,
      productName: safeProductName,
      qty: toInt(json['qty']),
      sellPrice: (json['sell_price'] ?? '0').toString(),
      lineAmount: resolveLineAmount(
        rawLineAmount: json['line_amount'],
        rawQty: json['qty'],
        rawSellPrice: json['sell_price'],
      ),
      returnedQty: toInt(json['returned_qty']),
    );
  }
}

String resolveLineAmount({
  required Object? rawLineAmount,
  required Object? rawQty,
  required Object? rawSellPrice,
}) {
  final String lineAmount = (rawLineAmount ?? '').toString().trim();
  if (lineAmount.isNotEmpty) {
    return lineAmount;
  }

  final int qty = toInt(rawQty);
  final double? sellPrice = double.tryParse((rawSellPrice ?? '').toString());
  if (qty <= 0 || sellPrice == null) {
    return '0.0000';
  }

  return (qty * sellPrice).toStringAsFixed(4);
}

int toInt(Object? value) {
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

List<SalesOrderData> toSalesOrderList(Object? value) {
  final List<Map<String, dynamic>> rawMaps =
      extractMapsByShape(value, looksLikeSalesOrderMap);

  return rawMaps.map(SalesOrderData.fromJson).toList();
}

List<SalesOrderItemData> toSalesOrderItemList(Object? value) {
  final List<Map<String, dynamic>> rawMaps =
      extractMapsByShape(value, looksLikeSalesOrderItemMap);

  return rawMaps.map(SalesOrderItemData.fromJson).toList();
}

typedef MapShapePredicate = bool Function(Map<String, dynamic> map);

List<Map<String, dynamic>> extractMapsByShape(
  Object? value,
  MapShapePredicate predicate, {
  int depth = 0,
}) {
  if (value == null || depth > 8) {
    return const <Map<String, dynamic>>[];
  }

  if (value is String) {
    final Object? decoded = tryDecodeJson(value);
    if (decoded == null) {
      return const <Map<String, dynamic>>[];
    }
    return extractMapsByShape(
      decoded,
      predicate,
      depth: depth + 1,
    );
  }

  if (value is Iterable) {
    final List<Map<String, dynamic>> result = <Map<String, dynamic>>[];

    for (final Object? item in value) {
      if (item is Map) {
        final map = toStringDynamicMap(item);
        if (predicate(map)) {
          result.add(map);
          continue;
        }
      }

      final nested = extractMapsByShape(
        item,
        predicate,
        depth: depth + 1,
      );
      if (nested.isNotEmpty) {
        result.addAll(nested);
      }
    }

    return result;
  }

  if (value is Map) {
    final map = toStringDynamicMap(value);
    if (predicate(map)) {
      return <Map<String, dynamic>>[map];
    }

    for (final String key in const <String>[
      'list',
      'items',
      'rows',
      'orders',
      'records',
      'result',
      'data',
      'value',
    ]) {
      if (!map.containsKey(key)) {
        continue;
      }

      final nested = extractMapsByShape(
        map[key],
        predicate,
        depth: depth + 1,
      );
      if (nested.isNotEmpty) {
        return nested;
      }
    }

    return extractMapsByShape(
      map.values,
      predicate,
      depth: depth + 1,
    );
  }

  return const <Map<String, dynamic>>[];
}

Map<String, dynamic> toStringDynamicMap(Map map) {
  return map.map(
    (key, value) => MapEntry(key.toString(), value),
  );
}

Object? tryDecodeJson(String raw) {
  final String trimmed = raw.trim();
  if (trimmed.isEmpty) {
    return null;
  }

  try {
    return jsonDecode(trimmed);
  } on FormatException {
    return null;
  }
}

bool looksLikeSalesOrderMap(Map<String, dynamic> map) {
  final hasBizNo = map.containsKey('biz_no');
  final hasOrderShape = map.containsKey('status') ||
      map.containsKey('id') ||
      map.containsKey('total_amount');
  return hasBizNo && hasOrderShape;
}

bool looksLikeSalesOrderItemMap(Map<String, dynamic> map) {
  return map.containsKey('product_id') &&
      (map.containsKey('qty') ||
          map.containsKey('returned_qty') ||
          map.containsKey('sell_price'));
}

int? toNullableInt(Object? value) {
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

// ── Sales Report (销售报告 / 热销排行) ──────────────────────────────────────────

class SalesReportItemData {
  const SalesReportItemData({
    required this.productId,
    required this.productName,
    required this.totalQty,
    required this.totalSales,
    required this.totalCost,
    required this.grossProfit,
  });

  final int productId;
  final String productName;
  final int totalQty;
  final String totalSales;
  final String totalCost;
  final String grossProfit;

  factory SalesReportItemData.fromJson(Map<String, dynamic> json) {
    final int id = toInt(json['product_id']);
    final String rawName = (json['product_name'] ?? '').toString().trim();
    return SalesReportItemData(
      productId: id,
      productName: rawName.isEmpty ? '商品#$id' : rawName,
      totalQty: toInt(json['total_qty']),
      totalSales: (json['total_sales'] ?? '0').toString(),
      totalCost: (json['total_cost'] ?? '0').toString(),
      grossProfit: (json['gross_profit'] ?? '0').toString(),
    );
  }

  double get totalSalesDouble => double.tryParse(totalSales) ?? 0.0;
  double get grossProfitDouble => double.tryParse(grossProfit) ?? 0.0;
}

class SalesReportSummaryData {
  const SalesReportSummaryData({
    required this.totalSales,
    required this.totalCost,
    required this.totalGrossProfit,
    required this.totalQty,
  });

  final String totalSales;
  final String totalCost;
  final String totalGrossProfit;
  final int totalQty;

  factory SalesReportSummaryData.fromJson(Map<String, dynamic> json) {
    return SalesReportSummaryData(
      totalSales: (json['total_sales'] ?? '0').toString(),
      totalCost: (json['total_cost'] ?? '0').toString(),
      totalGrossProfit: (json['total_gross_profit'] ?? '0').toString(),
      totalQty: toInt(json['total_qty']),
    );
  }

  double get totalSalesDouble => double.tryParse(totalSales) ?? 0.0;
  double get totalGrossProfitDouble =>
      double.tryParse(totalGrossProfit) ?? 0.0;
}

class SalesReportData {
  const SalesReportData({
    required this.groupBy,
    required this.startDate,
    required this.endDate,
    required this.list,
    required this.total,
    required this.page,
    required this.pageSize,
    required this.summary,
  });

  final String groupBy;
  final String startDate;
  final String endDate;
  final List<SalesReportItemData> list;
  final int total;
  final int page;
  final int pageSize;
  final SalesReportSummaryData summary;

  factory SalesReportData.fromJson(Map<String, dynamic> json) {
    final rawList = json['list'];
    final list = (rawList is List)
        ? rawList
            .whereType<Map<String, dynamic>>()
            .map(SalesReportItemData.fromJson)
            .toList()
        : <SalesReportItemData>[];

    final rawSummary = json['summary'];
    final summary = rawSummary is Map<String, dynamic>
        ? SalesReportSummaryData.fromJson(rawSummary)
        : const SalesReportSummaryData(
            totalSales: '0',
            totalCost: '0',
            totalGrossProfit: '0',
            totalQty: 0,
          );

    return SalesReportData(
      groupBy: (json['group_by'] ?? 'product').toString(),
      startDate: (json['start_date'] ?? '').toString(),
      endDate: (json['end_date'] ?? '').toString(),
      list: list,
      total: toInt(json['total']),
      page: toInt(json['page']),
      pageSize: toInt(json['page_size']),
      summary: summary,
    );
  }
}

// ── Trend (经营趋势) ───────────────────────────────────────────────────────────

class TrendDayData {
  const TrendDayData({
    required this.date,
    required this.totalSales,
    required this.totalGrossProfit,
    required this.totalOrders,
    required this.lowStockCount,
    required this.topSellingItem,
  });

  final String date;
  final String totalSales;
  final String totalGrossProfit;
  final int totalOrders;
  final int lowStockCount;
  final String topSellingItem;

  factory TrendDayData.fromJson(Map<String, dynamic> json) {
    return TrendDayData(
      date: (json['date'] ?? '').toString(),
      totalSales: (json['total_sales'] ?? '0').toString(),
      totalGrossProfit: (json['total_gross_profit'] ?? '0').toString(),
      totalOrders: toInt(json['total_orders']),
      lowStockCount: toInt(json['low_stock_count']),
      topSellingItem: (json['top_selling_item'] ?? '-').toString(),
    );
  }

  double get totalSalesDouble => double.tryParse(totalSales) ?? 0.0;
  double get totalGrossProfitDouble => double.tryParse(totalGrossProfit) ?? 0.0;

  double get grossProfitRate {
    final sales = totalSalesDouble;
    if (sales <= 0) return 0.0;
    return totalGrossProfitDouble / sales * 100;
  }
}

class TrendData {
  const TrendData({
    required this.startDate,
    required this.endDate,
    required this.days,
  });

  final String startDate;
  final String endDate;
  final List<TrendDayData> days;

  factory TrendData.fromJson(Map<String, dynamic> json) {
    final rawDays = json['days'];
    final List<TrendDayData> days = <TrendDayData>[];
    if (rawDays is List) {
      for (final item in rawDays) {
        if (item is Map<String, dynamic>) {
          days.add(TrendDayData.fromJson(item));
        }
      }
    }
    return TrendData(
      startDate: (json['start_date'] ?? '').toString(),
      endDate: (json['end_date'] ?? '').toString(),
      days: days,
    );
  }
}
