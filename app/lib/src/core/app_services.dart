import '../config/env.dart';
import '../features/auth/models/auth_repository.dart';
import '../features/dashboard/models/dashboard_repository.dart';
import '../network/api_client.dart';
import '../storage/session_storage.dart';

class AppServices {
  AppServices._({
    required this.sessionStorage,
    required this.apiClient,
    required this.authRepository,
    required this.dashboardRepository,
  });

  final SessionStorage sessionStorage;
  final ApiClient apiClient;
  final AuthRepository authRepository;
  final DashboardRepository dashboardRepository;

  factory AppServices.bootstrap() {
    final sessionStorage = SessionStorage();
    final apiClient = ApiClient(
      baseUrl: Env.apiBaseUrl,
      sessionStorage: sessionStorage,
    );

    return AppServices._(
      sessionStorage: sessionStorage,
      apiClient: apiClient,
      authRepository: AuthRepository(client: apiClient),
      dashboardRepository: DashboardRepository(client: apiClient),
    );
  }
}
