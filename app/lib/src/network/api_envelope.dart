class ApiEnvelope<T> {
  const ApiEnvelope({
    required this.code,
    required this.message,
    required this.data,
    required this.requestId,
  });

  final int code;
  final String message;
  final T data;
  final String requestId;

  factory ApiEnvelope.fromJson(
    Map<String, dynamic> json,
    T Function(Object? rawData) decoder,
  ) {
    return ApiEnvelope<T>(
      code: _toInt(json['code']),
      message: (json['message'] ?? '').toString(),
      data: decoder(json['data']),
      requestId: (json['request_id'] ?? '').toString(),
    );
  }

  static int _toInt(Object? value) {
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
}
