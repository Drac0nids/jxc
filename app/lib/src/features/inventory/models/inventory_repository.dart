import '../../../network/api_client.dart';
import 'inventory_models.dart';

class InventoryRepository {
  const InventoryRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  Future<ScanProductData> scanProduct(String barcode) async {
    final envelope = await _client.get<ScanProductData>(
      '/products/scan',
      query: <String, dynamic>{
        'barcode': barcode,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('scan product response data is invalid');
        }

        return ScanProductData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<InboundResultData> inbound(InboundRequest request) async {
    final envelope = await _client.post<InboundResultData>(
      '/inventory/inbound',
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('inbound response data is invalid');
        }

        return InboundResultData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<InboundBatchResultData> inboundBatch(
      List<InboundBatchItemRequest> items) async {
    final envelope = await _client.post<InboundBatchResultData>(
      '/inventory/inbound/batch',
      data: <String, dynamic>{
        'items':
            items.map((InboundBatchItemRequest item) => item.toJson()).toList(),
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('inbound batch response data is invalid');
        }

        return InboundBatchResultData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<OutboundResultData> outbound(OutboundRequest request) async {
    final envelope = await _client.post<OutboundResultData>(
      '/inventory/outbound',
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('outbound response data is invalid');
        }

        return OutboundResultData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<StockCheckData> createStockCheck(
      StockCheckCreateRequest request) async {
    final envelope = await _client.post<StockCheckData>(
      '/inventory/stock-checks',
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'create stock check response data is invalid');
        }

        return StockCheckData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<StockCheckData> getStockCheck(int id) async {
    final envelope = await _client.get<StockCheckData>(
      '/inventory/stock-checks/$id',
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'get stock check response data is invalid');
        }

        return StockCheckData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<StockCheckData> startStockCheck({
    required int id,
    int? expectedVersion,
  }) async {
    final payload = <String, dynamic>{
      if (expectedVersion != null) 'expected_version': expectedVersion,
    };

    final envelope = await _client.post<StockCheckData>(
      '/inventory/stock-checks/$id/start',
      data: payload,
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'start stock check response data is invalid');
        }

        return StockCheckData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<StockCheckData> confirmStockCheck({
    required int id,
    required StockCheckConfirmRequest request,
  }) async {
    final envelope = await _client.post<StockCheckData>(
      '/inventory/stock-checks/$id/confirm',
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'confirm stock check response data is invalid');
        }

        return StockCheckData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<PurchaseOrdersPageData> fetchPurchaseOrders({
    required String startDate,
    required String endDate,
    required int page,
    required int pageSize,
  }) async {
    final envelope = await _client.get<PurchaseOrdersPageData>(
      '/purchase-orders',
      query: <String, dynamic>{
        'start_date': startDate,
        'end_date': endDate,
        'page': page,
        'page_size': pageSize,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'purchase orders page response data is invalid');
        }

        return PurchaseOrdersPageData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<InboundLogsPageData> fetchInboundLogs({
    required String startDate,
    required String endDate,
    required int page,
    required int pageSize,
  }) async {
    final envelope = await _client.get<InboundLogsPageData>(
      '/inventory/logs',
      query: <String, dynamic>{
        'biz_type': 'IN_PURCHASE',
        'start_date': startDate,
        'end_date': endDate,
        'page': page,
        'page_size': pageSize,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'inbound logs page response data is invalid');
        }

        return InboundLogsPageData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<InboundLogsPageData> fetchStockCheckLogs({
    required String startDate,
    required String endDate,
    required int page,
    required int pageSize,
  }) async {
    final envelope = await _client.get<InboundLogsPageData>(
      '/inventory/logs',
      query: <String, dynamic>{
        'biz_type': 'ADJ_CHECK',
        'start_date': startDate,
        'end_date': endDate,
        'page': page,
        'page_size': pageSize,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'stock check logs page response data is invalid');
        }

        return InboundLogsPageData.fromJson(rawData);
      },
    );

    return envelope.data;
  }
}
