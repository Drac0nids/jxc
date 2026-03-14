class LoginRequest {
  const LoginRequest({
    required this.username,
    required this.password,
    this.tenantCode,
  });

  final String username;
  final String password;
  final String? tenantCode;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      'username': username,
      'password': password,
      if (tenantCode != null && tenantCode!.isNotEmpty)
        'tenant_code': tenantCode,
    };
  }
}
