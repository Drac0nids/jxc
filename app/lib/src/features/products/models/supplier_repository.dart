import '../../../network/api_client.dart';
import 'supplier_models.dart';

class SupplierRepository {
  const SupplierRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  Future<List<SupplierData>> listSuppliers({String? keyword}) async {
    final envelope = await _client.get<List<SupplierData>>(
      '/suppliers',
      query: keyword != null && keyword.trim().isNotEmpty
          ? <String, String>{'q': keyword.trim()}
          : null,
      decoder: (Object? raw) {
        if (raw is! List) return <SupplierData>[];
        return raw
            .whereType<Map<String, dynamic>>()
            .map(SupplierData.fromJson)
            .toList();
      },
    );
    return envelope.data;
  }

  Future<SupplierData> createSupplier(CreateSupplierRequest req) async {
    final envelope = await _client.post<SupplierData>(
      '/suppliers',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('invalid supplier response');
        }
        return SupplierData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  Future<SupplierData> updateSupplier(int id, UpdateSupplierRequest req) async {
    final envelope = await _client.put<SupplierData>(
      '/suppliers/$id',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('invalid supplier response');
        }
        return SupplierData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  Future<void> deleteSupplier(int id) async {
    await _client.delete<void>('/suppliers/$id', decoder: (_) {});
  }
}
