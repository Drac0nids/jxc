import 'dart:async';

import 'package:dio/dio.dart';
import 'package:uuid/uuid.dart';

import '../storage/session_storage.dart';
import 'api_envelope.dart';
import 'api_exception.dart';

class ApiClient {
  ApiClient({
    required String baseUrl,
    required SessionStorage sessionStorage,
    Dio? dio,
  })  : _sessionStorage = sessionStorage,
        _uuid = const Uuid(),
        _dio = dio ??
            Dio(
              BaseOptions(
                baseUrl: baseUrl,
                connectTimeout: const Duration(seconds: 15),
                receiveTimeout: const Duration(seconds: 15),
              ),
            ) {
    _dio.interceptors.add(
      InterceptorsWrapper(
        onRequest: _onRequest,
      ),
    );
  }

  final Dio _dio;
  final SessionStorage _sessionStorage;
  final Uuid _uuid;

  Future<ApiEnvelope<T>> get<T>(
    String path, {
    Map<String, dynamic>? query,
    required T Function(Object? rawData) decoder,
    bool authRequired = true,
  }) async {
    final response = await _request(
      method: 'GET',
      path: path,
      query: query,
      authRequired: authRequired,
    );
    return _decodeEnvelope<T>(response.data, decoder);
  }

  Future<ApiEnvelope<T>> post<T>(
    String path, {
    Object? data,
    Map<String, dynamic>? query,
    required T Function(Object? rawData) decoder,
    bool authRequired = true,
  }) async {
    final response = await _request(
      method: 'POST',
      path: path,
      data: data,
      query: query,
      authRequired: authRequired,
    );
    return _decodeEnvelope<T>(response.data, decoder);
  }

  Future<Response<dynamic>> _request({
    required String method,
    required String path,
    Object? data,
    Map<String, dynamic>? query,
    required bool authRequired,
  }) async {
    try {
      return await _dio.request<dynamic>(
        path,
        data: data,
        queryParameters: query,
        options: Options(
          method: method,
          extra: <String, dynamic>{
            _ExtraKeys.authRequired: authRequired,
            _ExtraKeys.method: method,
          },
        ),
      );
    } on DioException catch (error) {
      throw _normalizeDioException(error);
    }
  }

  FutureOr<void> _onRequest(
    RequestOptions options,
    RequestInterceptorHandler handler,
  ) async {
    final authRequired = (options.extra[_ExtraKeys.authRequired] as bool?) ?? true;
    final method = ((options.extra[_ExtraKeys.method] as String?) ?? options.method).toUpperCase();

    options.headers['x-request-id'] = _requestId();
    options.headers['x-client-type'] = 'android';

    if (_isWriteMethod(method)) {
      options.headers['x-idempotency-key'] = _idempotencyKey();
    }

    if (authRequired) {
      final session = await _sessionStorage.read();
      if (session != null && session.accessToken.isNotEmpty) {
        options.headers['Authorization'] = 'Bearer ${session.accessToken}';
      }
    }

    handler.next(options);
  }

  ApiEnvelope<T> _decodeEnvelope<T>(
    Object? raw,
    T Function(Object? rawData) decoder,
  ) {
    if (raw is! Map<String, dynamic>) {
      throw const ApiException(code: 5000, message: '响应格式错误');
    }

    final envelope = ApiEnvelope<T>.fromJson(raw, decoder);
    if (envelope.code != 200) {
      throw ApiException(
        code: envelope.code,
        message: envelope.message,
        requestId: envelope.requestId,
        data: raw['data'],
      );
    }

    return envelope;
  }

  ApiException _normalizeDioException(DioException error) {
    final responseData = error.response?.data;

    if (responseData is Map<String, dynamic>) {
      return ApiException(
        code: _toInt(responseData['code']),
        message: (responseData['message'] ?? error.message ?? '请求失败').toString(),
        requestId: responseData['request_id']?.toString(),
        data: responseData['data'],
      );
    }

    return ApiException(
      code: 5000,
      message: error.message ?? '网络请求失败',
      requestId: error.response?.headers.value('x-request-id'),
    );
  }

  int _toInt(Object? value) {
    if (value is int) {
      return value;
    }
    if (value is num) {
      return value.toInt();
    }
    if (value is String) {
      return int.tryParse(value) ?? 5000;
    }
    return 5000;
  }

  bool _isWriteMethod(String method) {
    return method == 'POST' || method == 'PUT' || method == 'PATCH' || method == 'DELETE';
  }

  String _requestId() {
    return 'req_${_uuid.v4().replaceAll('-', '')}';
  }

  String _idempotencyKey() {
    return _uuid.v4();
  }
}

class _ExtraKeys {
  const _ExtraKeys._();

  static const String authRequired = 'authRequired';
  static const String method = 'method';
}
