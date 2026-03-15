import '../../../network/api_client.dart';
import 'users_models.dart';

class UsersRepository {
  const UsersRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  // ── List users ────────────────────────────────────────────────────────────

  Future<List<UserData>> listUsers() async {
    final envelope = await _client.get<List<UserData>>(
      '/users',
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('list users response data is invalid');
        }
        final rawList = raw['list'];
        return (rawList is List)
            ? rawList
                .whereType<Map<String, dynamic>>()
                .map(UserData.fromJson)
                .toList()
            : <UserData>[];
      },
    );
    return envelope.data;
  }

  // ── Create user ───────────────────────────────────────────────────────────

  Future<UserData> createUser(CreateUserRequest request) async {
    final envelope = await _client.post<UserData>(
      '/users',
      data: request.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('create user response data is invalid');
        }
        return UserData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  // ── Update role ───────────────────────────────────────────────────────────

  Future<UserData> updateRole(String userId, UpdateUserRoleRequest req) async {
    final envelope = await _client.patch<UserData>(
      '/users/$userId/role',
      data: req.toJson(),
      decoder: (Object? raw) {
        if (raw is! Map<String, dynamic>) {
          throw const FormatException('update role response data is invalid');
        }
        return UserData.fromJson(raw);
      },
    );
    return envelope.data;
  }

  // ── Reset password ────────────────────────────────────────────────────────

  Future<void> resetPassword(String userId, ResetPasswordRequest req) async {
    await _client.post<Map<String, dynamic>>(
      '/users/$userId/reset-password',
      data: req.toJson(),
      decoder: (Object? raw) =>
          raw is Map<String, dynamic> ? raw : <String, dynamic>{},
    );
  }

  // ── Delete user ───────────────────────────────────────────────────────────

  Future<void> deleteUser(String userId) async {
    await _client.delete<Map<String, dynamic>>(
      '/users/$userId',
      decoder: (Object? raw) =>
          raw is Map<String, dynamic> ? raw : <String, dynamic>{},
    );
  }
}
