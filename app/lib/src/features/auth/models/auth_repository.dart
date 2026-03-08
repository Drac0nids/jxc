import '../../../core/models/user_session.dart';
import '../../../network/api_client.dart';
import 'login_request.dart';
import 'register_request.dart';

class AuthRepository {
  const AuthRepository({required ApiClient client}) : _client = client;

  final ApiClient _client;

  Future<UserSession> register(RegisterRequest request) async {
    final envelope = await _client.post<UserSession>(
      '/auth/register',
      authRequired: false,
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('register response data is invalid');
        }

        return UserSession.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<UserSession> login(LoginRequest request) async {
    final envelope = await _client.post<UserSession>(
      '/auth/login',
      authRequired: false,
      data: request.toJson(),
      decoder: (Object? rawData) {
        if (rawData is! Map<String, dynamic>) {
          throw const FormatException('login response data is invalid');
        }

        return UserSession.fromJson(rawData);
      },
    );

    return envelope.data;
  }

  Future<void> logout() async {
    await _client.post<Object?>(
      '/auth/logout',
      decoder: (_) => null,
    );
  }
}
