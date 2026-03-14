import '../../../network/api_client.dart';

// ── 数据模型 ───────────────────────────────────────────────────────────────────

enum BatchExpiryLevel {
  expired,   // 已过期
  critical,  // < 7天
  warning,   // < 15天
  notice,    // < 30天（或预警阈值内）
  ok,        // 不在预警期
}

class BatchData {
  const BatchData({
    required this.id,
    required this.productId,
    required this.lotNumber,
    this.supplier,
    required this.inboundAt,
    this.producedAt,
    this.expiresAt,
    this.notes,
    required this.isSoldOut,
    this.soldOutAt,
    this.daysUntilExpiry,
    this.expiryLevel,
    required this.createdAt,
  });

  final int id;
  final int productId;
  final String lotNumber;
  final String? supplier;
  final DateTime inboundAt;
  final DateTime? producedAt;
  final DateTime? expiresAt;
  final String? notes;
  final bool isSoldOut;
  final DateTime? soldOutAt;
  final int? daysUntilExpiry;
  final BatchExpiryLevel? expiryLevel;
  final DateTime createdAt;

  factory BatchData.fromJson(Map<String, dynamic> json) {
    BatchExpiryLevel? level;
    final rawLevel = json['expiry_level'] as String?;
    if (rawLevel != null) {
      level = switch (rawLevel) {
        'EXPIRED' => BatchExpiryLevel.expired,
        'CRITICAL' => BatchExpiryLevel.critical,
        'WARNING' => BatchExpiryLevel.warning,
        'NOTICE' => BatchExpiryLevel.notice,
        _ => null,
      };
    }
    return BatchData(
      id: json['id'] as int,
      productId: json['product_id'] as int,
      lotNumber: json['lot_number'] as String,
      supplier: json['supplier'] as String?,
      inboundAt: DateTime.parse(json['inbound_at'] as String),
      producedAt: json['produced_at'] != null
          ? DateTime.parse(json['produced_at'] as String)
          : null,
      expiresAt: json['expires_at'] != null
          ? DateTime.parse(json['expires_at'] as String)
          : null,
      notes: json['notes'] as String?,
      isSoldOut: json['is_sold_out'] as bool,
      soldOutAt: json['sold_out_at'] != null
          ? DateTime.parse(json['sold_out_at'] as String)
          : null,
      daysUntilExpiry: json['days_until_expiry'] as int?,
      expiryLevel: level,
      createdAt: DateTime.parse(json['created_at'] as String),
    );
  }
}

class ExpiringBatchData {
  const ExpiringBatchData({
    required this.batch,
    required this.productName,
    required this.productSku,
  });

  final BatchData batch;
  final String productName;
  final String productSku;

  factory ExpiringBatchData.fromJson(Map<String, dynamic> json) {
    return ExpiringBatchData(
      batch: BatchData.fromJson(json['batch'] as Map<String, dynamic>),
      productName: json['product_name'] as String,
      productSku: json['product_sku'] as String,
    );
  }
}

// ── 请求体 ─────────────────────────────────────────────────────────────────────

class CreateBatchRequest {
  const CreateBatchRequest({
    required this.productId,
    this.lotNumber,
    this.supplier,
    this.inboundAt,
    this.producedAt,
    this.expiresAt,
    this.notes,
  });

  final int productId;
  final String? lotNumber;
  final String? supplier;
  final DateTime? inboundAt;
  final DateTime? producedAt;
  final DateTime? expiresAt;
  final String? notes;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'product_id': productId,
        if (lotNumber != null) 'lot_number': lotNumber,
        if (supplier != null) 'supplier': supplier,
        if (inboundAt != null)
          'inbound_at': inboundAt!.toIso8601String().substring(0, 10),
        if (producedAt != null)
          'produced_at': producedAt!.toIso8601String().substring(0, 10),
        if (expiresAt != null)
          'expires_at': expiresAt!.toIso8601String().substring(0, 10),
        if (notes != null) 'notes': notes,
      };
}

class UpdateBatchRequest {
  const UpdateBatchRequest({
    this.lotNumber,
    this.supplier,
    this.producedAt,
    this.expiresAt,
    this.notes,
  });

  final String? lotNumber;
  final String? supplier;
  final DateTime? producedAt;
  final DateTime? expiresAt;
  final String? notes;

  Map<String, dynamic> toJson() => <String, dynamic>{
        if (lotNumber != null) 'lot_number': lotNumber,
        'supplier': supplier,
        'produced_at': producedAt?.toIso8601String().substring(0, 10),
        'expires_at': expiresAt?.toIso8601String().substring(0, 10),
        if (notes != null) 'notes': notes,
      };
}

// ── Repository ─────────────────────────────────────────────────────────────────

class BatchRepository {
  const BatchRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  Future<List<BatchData>> listBatches({
    int? productId,
    bool onlyActive = false,
  }) async {
    final envelope = await _client.get<List<BatchData>>(
      '/batches',
      query: <String, dynamic>{
        if (productId != null) 'product_id': productId,
        if (onlyActive) 'only_active': true,
      },
      decoder: (Object? raw) {
        if (raw is! List) throw const FormatException('invalid batch list');
        return raw
            .whereType<Map<String, dynamic>>()
            .map(BatchData.fromJson)
            .toList();
      },
    );
    return envelope.data;
  }

  Future<List<ExpiringBatchData>> listExpiringBatches({
    int withinDays = 30,
  }) async {
    final envelope = await _client.get<List<ExpiringBatchData>>(
      '/batches/expiring',
      query: <String, dynamic>{'within_days': withinDays},
      decoder: (Object? raw) {
        if (raw is! List) throw const FormatException('invalid expiring list');
        return raw
            .whereType<Map<String, dynamic>>()
            .map(ExpiringBatchData.fromJson)
            .toList();
      },
    );
    return envelope.data;
  }

  Future<BatchData> createBatch(CreateBatchRequest req) async {
    final envelope = await _client.post<BatchData>(
      '/batches',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('invalid batch response');
        }
        return BatchData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  Future<BatchData> updateBatch(int id, UpdateBatchRequest req) async {
    final envelope = await _client.put<BatchData>(
      '/batches/$id',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('invalid batch response');
        }
        return BatchData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  Future<BatchData> markSoldOut(int id) async {
    final envelope = await _client.post<BatchData>(
      '/batches/$id/sold-out',
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('invalid batch response');
        }
        return BatchData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  Future<void> deleteBatch(int id) async {
    await _client.delete<void>('/batches/$id', decoder: (_) {});
  }
}
