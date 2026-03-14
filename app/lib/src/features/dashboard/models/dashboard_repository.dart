import '../../../network/api_client.dart';
import 'dashboard_data.dart';

class DashboardRepository {
  const DashboardRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  Future<DashboardData> fetchDashboard({String? date}) async {
    final query = <String, dynamic>{};
    if (date != null && date.trim().isNotEmpty) {
      query['date'] = date.trim();
    }

    final envelope = await _client.get<DashboardData>(
      '/reports/dashboard',
      query: query.isEmpty ? null : query,
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('dashboard response data is invalid');
        }

        return DashboardData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<DashboardOrdersDrilldownData> fetchDashboardOrdersDrilldown({
    required String startDate,
    required String endDate,
    required int page,
    required int pageSize,
  }) async {
    final envelope = await _client.get<DashboardOrdersDrilldownData>(
      '/reports/dashboard/orders',
      query: <String, dynamic>{
        'start_date': startDate,
        'end_date': endDate,
        'page': page,
        'page_size': pageSize,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'dashboard orders drilldown response data is invalid');
        }

        return DashboardOrdersDrilldownData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<TrendData> fetchTrend({
    required String startDate,
    required String endDate,
  }) async {
    final envelope = await _client.get<TrendData>(
      '/reports/trend',
      query: <String, dynamic>{
        'start_date': startDate,
        'end_date': endDate,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('trend response data is invalid');
        }
        return TrendData.fromJson(rawData);
      },
    );
    return envelope.data;
  }

  Future<SalesReportData> fetchSalesReport({
    required String startDate,
    required String endDate,
    int page = 1,
    int pageSize = 50,
  }) async {
    final envelope = await _client.get<SalesReportData>(
      '/reports/sales',
      query: <String, dynamic>{
        'start_date': startDate,
        'end_date': endDate,
        'group_by': 'product',
        'page': page,
        'page_size': pageSize,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'sales report response data is invalid');
        }
        return SalesReportData.fromJson(rawData);
      },
    );
    return envelope.data;
  }
}
