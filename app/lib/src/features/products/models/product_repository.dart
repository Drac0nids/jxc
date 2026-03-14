import '../../../network/api_client.dart';
import 'product_models.dart';

class ProductRepository {
  const ProductRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  Future<PagedData<ProductData>> listProducts(ListProductsQuery query) async {
    final envelope = await _client.get<PagedData<ProductData>>(
      '/products',
      query: query.toQuery(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('list products response data is invalid');
        }

        final rawList = rawData['list'];
        final list = (rawList is List)
            ? rawList
                .whereType<Map<String, dynamic>>()
                .map(ProductData.fromJson)
                .toList()
            : <ProductData>[];

        return PagedData<ProductData>(
          list: list,
          total: _toInt(rawData['total']),
          page: _toInt(rawData['page']),
          pageSize: _toInt(rawData['page_size']),
        );
      },
    );

    return envelope.data;
  }

  Future<ProductData> getProduct(int id) async {
    final envelope = await _client.get<ProductData>(
      '/products/$id',
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('get product response data is invalid');
        }

        return ProductData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<ProductData> createProduct(CreateProductRequest request) async {
    final envelope = await _client.post<ProductData>(
      '/products',
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'create product response data is invalid');
        }

        return ProductData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<ProductData> updateProduct({
    required int id,
    required UpdateProductRequest request,
  }) async {
    final envelope = await _client.put<ProductData>(
      '/products/$id',
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'update product response data is invalid');
        }

        return ProductData.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<DeleteProductResult> deleteProduct({
    required int id,
    int? expectedVersion,
  }) async {
    final envelope = await _client.delete<DeleteProductResult>(
      '/products/$id',
      query: <String, dynamic>{
        if (expectedVersion != null) 'expected_version': expectedVersion,
      },
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException(
              'delete product response data is invalid');
        }

        return DeleteProductResult.fromJson(rawData);
      },
    );

    return envelope.data;
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
