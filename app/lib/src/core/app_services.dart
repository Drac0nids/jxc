import '../config/env.dart';
import '../features/auth/models/auth_repository.dart';
import '../features/dashboard/models/dashboard_repository.dart';
import '../features/inventory/models/inventory_repository.dart';
import '../features/products/models/batch_models.dart';
import '../features/products/models/category_repository.dart';
import '../features/products/models/product_repository.dart';
import '../features/users/models/users_repository.dart';
import '../network/api_client.dart';
import '../storage/session_storage.dart';

class AppServices {
  AppServices._({
    required this.sessionStorage,
    required this.apiClient,
    required this.authRepository,
    required this.dashboardRepository,
    required this.inventoryRepository,
    required this.productRepository,
    required this.categoryRepository,
    required this.batchRepository,
    required this.usersRepository,
  });

  final SessionStorage sessionStorage;
  final ApiClient apiClient;
  final AuthRepository authRepository;
  final DashboardRepository dashboardRepository;
  final InventoryRepository inventoryRepository;
  final ProductRepository productRepository;
  final CategoryRepository categoryRepository;
  final BatchRepository batchRepository;
  final UsersRepository usersRepository;

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
      inventoryRepository: InventoryRepository(client: apiClient),
      productRepository: ProductRepository(client: apiClient),
      categoryRepository: CategoryRepository(client: apiClient),
      batchRepository: BatchRepository(client: apiClient),
      usersRepository: UsersRepository(client: apiClient),
    );
  }
}
