import 'auth_user.dart';

class UserSession {
  const UserSession({
    required this.accessToken,
    required this.refreshToken,
    required this.expiresIn,
    required this.tenantId,
    required this.tenantCode,
    required this.user,
  });

  final String accessToken;
  final String refreshToken;
  final int expiresIn;
  final String tenantId;
  /// 人类可读源户码，登录后保存，下次自动填写
  final String tenantCode;
  final AuthUser user;

  factory UserSession.fromJson(Map<String, dynamic> json) {
    return UserSession(
      accessToken: (json['access_token'] ?? '').toString(),
      refreshToken: (json['refresh_token'] ?? '').toString(),
      expiresIn: _toInt(json['expires_in']),
      tenantId: (json['tenant_id'] ?? '').toString(),
      tenantCode: (json['tenant_code'] ?? '').toString(),
      user: AuthUser.fromJson(
          (json['user_info'] ?? <String, dynamic>{}) as Map<String, dynamic>),
    );
  }

  factory UserSession.fromStorageJson(Map<String, dynamic> json) {
    return UserSession(
      accessToken: (json['accessToken'] ?? '').toString(),
      refreshToken: (json['refreshToken'] ?? '').toString(),
      expiresIn: _toInt(json['expiresIn']),
      tenantId: (json['tenantId'] ?? '').toString(),
      tenantCode: (json['tenantCode'] ?? '').toString(),
      user: AuthUser.fromJson(
          (json['user'] ?? <String, dynamic>{}) as Map<String, dynamic>),
    );
  }

  Map<String, dynamic> toStorageJson() {
    return <String, dynamic>{
      'accessToken': accessToken,
      'refreshToken': refreshToken,
      'expiresIn': expiresIn,
      'tenantId': tenantId,
      'tenantCode': tenantCode,
      'user': user.toJson(),
    };
  }

  static int _toInt(Object? value) {
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
}
