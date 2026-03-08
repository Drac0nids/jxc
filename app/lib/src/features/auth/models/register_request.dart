class RegisterRequest {
  const RegisterRequest({
    required this.username,
    required this.name,
    required this.password,
    this.tenantName,
  });

  final String username;
  final String name;
  final String password;
  final String? tenantName;

  Map<String, dynamic> toJson() {
    return <String, dynamic>{
      'tenant_name': tenantName,
      'username': username,
      'name': name,
      'password': password,
    };
  }
}
