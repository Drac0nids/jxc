class ApiException implements Exception {
  const ApiException({
    required this.code,
    required this.message,
    this.requestId,
    this.data,
  });

  final int code;
  final String message;
  final String? requestId;
  final Object? data;

  @override
  String toString() {
    if (requestId == null || requestId!.isEmpty) {
      return 'ApiException(code=$code, message=$message)';
    }

    return 'ApiException(code=$code, message=$message, request_id=$requestId)';
  }
}
