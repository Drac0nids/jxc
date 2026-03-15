import '../../../network/api_client.dart';
import 'serial_models.dart';

class SerialRepository {
  const SerialRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  /// 序列号入库
  Future<SerialInboundResult> inbound(SerialInboundRequest req) async {
    final envelope = await _client.post<SerialInboundResult>(
      '/serials/inbound',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('serial inbound response invalid');
        }
        return SerialInboundResult.fromJson(raw);
      },
    );
    return envelope.data;
  }

  /// 序列号出库
  Future<SerialOutboundResult> outbound(SerialOutboundRequest req) async {
    final envelope = await _client.post<SerialOutboundResult>(
      '/serials/outbound',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('serial outbound response invalid');
        }
        return SerialOutboundResult.fromJson(raw);
      },
    );
    return envelope.data;
  }

  /// 按 SN 查询
  Future<SerialNumber?> findBySn(String sn) async {
    final envelope = await _client.get<SerialNumber?>(
      '/serials',
      query: <String, dynamic>{'sn': sn},
      decoder: (Object? raw) {
        if (raw == null || raw is! Map<String, dynamic>) return null;
        return SerialNumber.fromJson(raw);
      },
    );
    return envelope.data;
  }

  /// 按商品列出在库 SN
  Future<List<SerialNumber>> listByProduct(int productId,
      {String? status}) async {
    final envelope = await _client.get<List<SerialNumber>>(
      '/serials',
      query: <String, dynamic>{
        'product_id': productId,
        if (status != null) 'status': status,
      },
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) return <SerialNumber>[];
        final list = raw['list'];
        return (list is List)
            ? list
                .whereType<Map<String, dynamic>>()
                .map(SerialNumber.fromJson)
                .toList()
            : <SerialNumber>[];
      },
    );
    return envelope.data;
  }
  /// 查询 SN 历史记录（全局，分页）
  Future<SerialHistoryResult> fetchHistory({
    String? status,
    int page = 1,
    int pageSize = 20,
  }) async {
    final envelope = await _client.get<SerialHistoryResult>(
      '/serials/history',
      query: <String, dynamic>{
        if (status != null) 'status': status,
        'page': page,
        'page_size': pageSize,
      },
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          return SerialHistoryResult(list: [], total: 0, page: page, pageSize: pageSize);
        }
        return SerialHistoryResult.fromJson(raw);
      },
    );
    return envelope.data;
  }
}
