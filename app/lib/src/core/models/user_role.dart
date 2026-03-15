enum UserRole {
  owner,
  admin,
  purchaser,
  sales,
  unknown;

  static UserRole fromServer(String value) {
    switch (value.toUpperCase()) {
      case 'OWNER':
        return UserRole.owner;
      case 'ADMIN':
        return UserRole.admin;
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
      case UserRole.admin:
        return 'ADMIN';
      case UserRole.purchaser:
        return 'PURCHASER';
      case UserRole.sales:
        return 'SALES';
      case UserRole.unknown:
        return 'UNKNOWN';
    }
  }

  String get chineseLabel {
    switch (this) {
      case UserRole.owner:
        return '老板';
      case UserRole.admin:
        return '副管理员';
      case UserRole.purchaser:
        return '采购员';
      case UserRole.sales:
        return '销售员';
      case UserRole.unknown:
        return '未知角色';
    }
  }
}
