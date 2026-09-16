import 'package:flutter_test/flutter_test.dart';
import 'package:jxc_app/src/core/models/user_role.dart';

void main() {
  group('UserRole.fromServer', () {
    test('识别服务端返回的全部角色', () {
      expect(UserRole.fromServer('OWNER'), UserRole.owner);
      expect(UserRole.fromServer('ADMIN'), UserRole.admin);
      expect(UserRole.fromServer('PURCHASER'), UserRole.purchaser);
      expect(UserRole.fromServer('SALES'), UserRole.sales);
    });

    test('大小写不敏感', () {
      expect(UserRole.fromServer('owner'), UserRole.owner);
      expect(UserRole.fromServer('Owner'), UserRole.owner);
      expect(UserRole.fromServer('purchaser'), UserRole.purchaser);
    });

    test('未知角色回退到 unknown，不抛异常', () {
      expect(UserRole.fromServer(''), UserRole.unknown);
      expect(UserRole.fromServer('AUDITOR'), UserRole.unknown);
    });
  });

  group('UserRole.serverValue', () {
    test('与服务端常量一致，且 fromServer(serverValue) 可往返', () {
      const expected = <UserRole, String>{
        UserRole.owner: 'OWNER',
        UserRole.admin: 'ADMIN',
        UserRole.purchaser: 'PURCHASER',
        UserRole.sales: 'SALES',
        UserRole.unknown: 'UNKNOWN',
      };

      expected.forEach((UserRole role, String wire) {
        expect(role.serverValue, wire);
        expect(UserRole.fromServer(role.serverValue), role);
      });
    });
  });

  group('UserRole.chineseLabel', () {
    test('每个角色都有中文名，且互不重复', () {
      final List<String> labels =
          UserRole.values.map((UserRole r) => r.chineseLabel).toList();

      expect(labels.any((String l) => l.trim().isEmpty), isFalse);
      expect(labels.toSet().length, labels.length);
    });
  });
}
