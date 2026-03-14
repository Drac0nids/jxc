import 'dart:async';
import 'dart:convert';

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

  Future<ApiEnvelope<T>> put<T>(
    String path, {
    Object? data,
    Map<String, dynamic>? query,
    required T Function(Object? rawData) decoder,
    bool authRequired = true,
  }) async {
    final response = await _request(
      method: 'PUT',
      path: path,
      data: data,
      query: query,
      authRequired: authRequired,
    );
    return _decodeEnvelope<T>(response.data, decoder);
  }

  Future<ApiEnvelope<T>> patch<T>(
    String path, {
    Object? data,
    Map<String, dynamic>? query,
    required T Function(Object? rawData) decoder,
    bool authRequired = true,
  }) async {
    final response = await _request(
      method: 'PATCH',
      path: path,
      data: data,
      query: query,
      authRequired: authRequired,
    );
    return _decodeEnvelope<T>(response.data, decoder);
  }

  Future<ApiEnvelope<T>> delete<T>(
    String path, {
    Object? data,
    Map<String, dynamic>? query,
    required T Function(Object? rawData) decoder,
    bool authRequired = true,
  }) async {
    final response = await _request(
      method: 'DELETE',
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
    final authRequired =
        (options.extra[_ExtraKeys.authRequired] as bool?) ?? true;
    final method =
        ((options.extra[_ExtraKeys.method] as String?) ?? options.method)
            .toUpperCase();

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
    final Map<String, dynamic>? payload = _asJsonMap(raw);
    if (payload == null) {
      throw ApiException(
        code: _ClientErrorCodes.invalidResponse,
        message: '响应格式错误：期望 JSON 对象',
        data: <String, dynamic>{
          'raw_type': raw == null ? 'null' : raw.runtimeType.toString(),
        },
      );
    }

    final envelope = ApiEnvelope<T>.fromJson(payload, decoder);
    if (envelope.code != 200) {
      throw ApiException(
        code: envelope.code,
        message: envelope.message,
        requestId: envelope.requestId,
        data: payload['data'],
      );
    }

    return envelope;
  }

  ApiException _normalizeDioException(DioException error) {
    final response = error.response;
    final responseData = response?.data;
    final requestId = response?.headers.value('x-request-id');
    final responseMap = _asJsonMap(responseData);

    if (responseMap != null) {
      return ApiException(
        code: _toInt(responseMap['code']),
        message: (responseMap['message'] ?? error.message ?? '请求失败').toString(),
        requestId: responseMap['request_id']?.toString() ?? requestId,
        data: responseMap['data'],
      );
    }

    if (_isNetworkException(error)) {
      return ApiException(
        code: _ClientErrorCodes.network,
        message: _networkErrorMessage(error),
        requestId: requestId,
        data: _buildDiagnostics(error),
      );
    }

    final int? statusCode = response?.statusCode;
    final String statusText = statusCode == null ? '未知状态码' : 'HTTP $statusCode';

    return ApiException(
      code: _ClientErrorCodes.invalidResponse,
      message: '请求失败：$statusText，响应不是标准 JSON',
      requestId: requestId,
      data: _buildDiagnostics(error),
    );
  }

  Map<String, dynamic>? _asJsonMap(Object? raw) {
    if (raw is Map<String, dynamic>) {
      return raw;
    }

    if (raw is Map) {
      return raw.map(
        (key, value) => MapEntry(key.toString(), value),
      );
    }

    if (raw is String && raw.trim().isNotEmpty) {
      try {
        final decoded = jsonDecode(raw);
        if (decoded is Map<String, dynamic>) {
          return decoded;
        }
        if (decoded is Map) {
          return decoded.map(
            (key, value) => MapEntry(key.toString(), value),
          );
        }
      } on FormatException {
        return null;
      }
    }

    return null;
  }

  bool _isNetworkException(DioException error) {
    return error.type == DioExceptionType.connectionTimeout ||
        error.type == DioExceptionType.sendTimeout ||
        error.type == DioExceptionType.receiveTimeout ||
        error.type == DioExceptionType.connectionError ||
        error.type == DioExceptionType.badCertificate;
  }

  String _networkErrorMessage(DioException error) {
    final rawMessage = (error.message ?? '').trim();

    if (rawMessage.contains('CLEARTEXT communication')) {
      return '网络连接被系统拦截（HTTP 明文请求受限）';
    }

    if (error.type == DioExceptionType.connectionTimeout) {
      return '连接服务器超时，请检查网络或服务器地址';
    }
    if (error.type == DioExceptionType.receiveTimeout) {
      return '服务器响应超时，请稍后重试';
    }
    if (error.type == DioExceptionType.sendTimeout) {
      return '请求发送超时，请检查网络后重试';
    }
    if (error.type == DioExceptionType.badCertificate) {
      return '服务器证书校验失败';
    }

    if (rawMessage.isNotEmpty) {
      return '网络请求失败：$rawMessage';
    }

    return '网络请求失败，请检查网络连接';
  }

  Map<String, dynamic> _buildDiagnostics(DioException error) {
    final responseData = error.response?.data;

    return <String, dynamic>{
      'dio_type': error.type.name,
      'method': error.requestOptions.method,
      'url': error.requestOptions.uri.toString(),
      if (error.response?.statusCode != null)
        'http_status': error.response?.statusCode,
      if (error.message != null && error.message!.isNotEmpty)
        'dio_message': error.message,
      if (responseData != null)
        'response_preview': _responsePreview(responseData),
    };
  }

  Object _responsePreview(Object data) {
    if (data is String) {
      const int maxLen = 500;
      if (data.length <= maxLen) {
        return data;
      }
      return '${data.substring(0, maxLen)}...';
    }

    return data;
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
    return method == 'POST' ||
        method == 'PUT' ||
        method == 'PATCH' ||
        method == 'DELETE';
  }

  String _requestId() {
    return 'req_${_uuid.v4().replaceAll('-', '')}';
  }

  String _idempotencyKey() {
    return _uuid.v4();
  }
}

class _ClientErrorCodes {
  const _ClientErrorCodes._();

  static const int network = 9000;
  static const int invalidResponse = 9001;
}

class _ExtraKeys {
  const _ExtraKeys._();

  static const String authRequired = 'authRequired';
  static const String method = 'method';
}
