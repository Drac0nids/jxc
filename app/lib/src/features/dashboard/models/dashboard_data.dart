class DashboardData {
  const DashboardData({
    required this.totalSales,
    required this.totalGrossProfit,
    required this.totalOrders,
    required this.lowStockCount,
    required this.topSellingItem,
  });

  final String totalSales;
  final String totalGrossProfit;
  final int totalOrders;
  final int lowStockCount;
  final String topSellingItem;

  factory DashboardData.fromJson(Map<String, dynamic> json) {
    return DashboardData(
      totalSales: (json['total_sales'] ?? '0').toString(),
      totalGrossProfit: (json['total_gross_profit'] ?? '0').toString(),
      totalOrders: _toInt(json['total_orders']),
      lowStockCount: _toInt(json['low_stock_count']),
      topSellingItem: (json['top_selling_item'] ?? '-').toString(),
    );
  }

  static int _toInt(Object? value) {
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
}
