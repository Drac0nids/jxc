enum UserRole {
  owner,
  purchaser,
  sales,
  unknown;

  static UserRole fromServer(String value) {
    switch (value.toUpperCase()) {
      case 'OWNER':
        return UserRole.owner;
      case 'PURCHASER':
        return UserRole.purchaser;
      case 'SALES':
        return UserRole.sales;
      default:
        return UserRole.unknown;
    }
  }

  String get serverValue {
    switch (this) {
      case UserRole.owner:
        return 'OWNER';
      case UserRole.purchaser:
        return 'PURCHASER';
      case UserRole.sales:
        return 'SALES';
      case UserRole.unknown:
        return 'UNKNOWN';
    }
  }
}
