import 'package:uuid/uuid.dart';

// ── SerialNumber 数据模型 ─────────────────────────────────────────────────────

class SerialNumber {
  const SerialNumber({
    required this.id,
    required this.sn,
    required this.productId,
    this.batchId,
    required this.status,
    this.unitCost,
    this.sellPrice,
    this.inboundBizNo,
    this.outboundBizNo,
  });

  final String id;
  final String sn;
  final int productId;
  final String? batchId;
  final String status; // IN_STOCK | SOLD | RETURNED
  final String? unitCost;
  final String? sellPrice;
  final String? inboundBizNo;
  final String? outboundBizNo;

  bool get isInStock => status == 'IN_STOCK';

  factory SerialNumber.fromJson(Map<String, dynamic> json) => SerialNumber(
        id: json['id']?.toString() ?? '',
        sn: json['sn']?.toString() ?? '',
        productId: _toInt(json['product_id']),
        batchId: json['batch_id']?.toString(),
        status: json['status']?.toString() ?? 'IN_STOCK',
        unitCost: json['unit_cost']?.toString(),
        sellPrice: json['sell_price']?.toString(),
        inboundBizNo: json['inbound_biz_no']?.toString(),
        outboundBizNo: json['outbound_biz_no']?.toString(),
      );
}

// ── 入库请求 ──────────────────────────────────────────────────────────────────

class SerialInboundRequest {
  const SerialInboundRequest({
    required this.productId,
    this.batchId,
    this.unitCost,
    required this.sns,
  });

  final int productId;
  final String? batchId;
  final String? unitCost;
  final List<String> sns;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'product_id': productId,
        if (batchId != null) 'batch_id': batchId,
        if (unitCost != null && unitCost!.trim().isNotEmpty)
          'unit_cost': unitCost!.trim(),
        'sns': sns,
      };
}

// ── 出库请求 ──────────────────────────────────────────────────────────────────

class SerialOutboundRequest {
  const SerialOutboundRequest({
    this.sellPrice,
    required this.sns,
  });

  final String? sellPrice;
  final List<String> sns;

  Map<String, dynamic> toJson() => <String, dynamic>{
        if (sellPrice != null && sellPrice!.trim().isNotEmpty)
          'sell_price': sellPrice!.trim(),
        'sns': sns,
      };
}

// ── 入库结果 ──────────────────────────────────────────────────────────────────

class SerialInboundResult {
  const SerialInboundResult({required this.bizNo, required this.count});

  final String bizNo;
  final int count;

  factory SerialInboundResult.fromJson(Map<String, dynamic> json) =>
      SerialInboundResult(
        bizNo: json['biz_no']?.toString() ?? '',
        count: _toInt(json['count']),
      );
}

// ── 出库结果 ──────────────────────────────────────────────────────────────────

class SerialOutboundResult {
  const SerialOutboundResult({
    required this.bizNo,
    required this.count,
    required this.list,
  });

  final String bizNo;
  final int count;
  final List<SerialNumber> list;

  factory SerialOutboundResult.fromJson(Map<String, dynamic> json) =>
      SerialOutboundResult(
        bizNo: json['biz_no']?.toString() ?? '',
        count: _toInt(json['count']),
        list: (json['list'] as List<dynamic>? ?? <dynamic>[])
            .whereType<Map<String, dynamic>>()
            .map(SerialNumber.fromJson)
            .toList(),
      );
}

// ── 扫码会话中的临时条目 ──────────────────────────────────────────────────────

class SerialScanEntry {
  SerialScanEntry({
    required this.sn,
    this.productName,
    this.status = 'pending',
    this.errorMsg,
  }) : scannedAt = DateTime.now();

  final String sn;
  final String? productName;
  final String status; // pending | success | error
  final String? errorMsg;
  final DateTime scannedAt;

  bool get isSuccess => status == 'success';
  bool get isError => status == 'error';
}

// ── 幂等键生成 ────────────────────────────────────────────────────────────────

String generateSerialBizKey() => const Uuid().v4();

int _toInt(Object? v) {
  if (v is int) return v;
  if (v is num) return v.toInt();
  if (v is String) return int.tryParse(v) ?? 0;
  return 0;
}
