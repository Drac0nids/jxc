// ════════════════════════════════════════════════════════════════════════════
// Users Models
// ════════════════════════════════════════════════════════════════════════════

class UserData {
  const UserData({
    required this.id,
    required this.username,
    required this.name,
    required this.role,
  });

  final String id;
  final String username;
  final String name;
  final String role; // "OWNER" | "PURCHASER" | "SALES"

  factory UserData.fromJson(Map<String, dynamic> json) {
    return UserData(
      id: (json['id'] ?? '').toString(),
      username: (json['username'] ?? '').toString(),
      name: (json['name'] ?? '').toString(),
      role: (json['role'] ?? '').toString().toUpperCase(),
    );
  }

  String get chineseRole {
    switch (role) {
      case 'OWNER':
        return '老板';
      case 'PURCHASER':
        return '采购员';
      case 'SALES':
        return '销售员';
      default:
        return role;
    }
  }
}

// ── Request bodies ────────────────────────────────────────────────────────

class CreateUserRequest {
  const CreateUserRequest({
    required this.username,
    required this.name,
    required this.password,
    required this.role,
  });

  final String username;
  final String name;
  final String password;
  final String role;

  Map<String, dynamic> toJson() => <String, dynamic>{
        'username': username,
        'name': name,
        'password': password,
        'role': role,
      };
}

class UpdateUserRoleRequest {
  const UpdateUserRoleRequest({required this.role});
  final String role;
  Map<String, dynamic> toJson() => <String, dynamic>{'role': role};
}

class ResetPasswordRequest {
  const ResetPasswordRequest({required this.newPassword});
  final String newPassword;
  Map<String, dynamic> toJson() =>
      <String, dynamic>{'new_password': newPassword};
}
