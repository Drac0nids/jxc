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
}
