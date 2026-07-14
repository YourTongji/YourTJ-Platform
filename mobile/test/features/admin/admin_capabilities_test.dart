import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/admin/domain/admin_capabilities.dart';

void main() {
  test('unknown or empty capabilities never expose a management module', () {
    expect(adminModulesForCapabilities(const <String>[]), isEmpty);
    expect(
      adminModulesForCapabilities(const <String>['future.unknown']),
      isEmpty,
    );
  });

  test('each section is derived from exact server capabilities', () {
    final List<AdminModule> modules =
        adminModulesForCapabilities(const <String>[
          AdminCapabilities.moderateContent,
          AdminCapabilities.reviewAppeals,
          AdminCapabilities.runOperations,
        ]);

    expect(
      modules.map((AdminModule module) => module.section),
      containsAll(<AdminSection>[
        AdminSection.moderation,
        AdminSection.appeals,
        AdminSection.resources,
        AdminSection.system,
      ]),
    );
    expect(
      modules.map((AdminModule module) => module.section),
      isNot(contains(AdminSection.users)),
    );
    expect(
      modules.map((AdminModule module) => module.section),
      isNot(contains(AdminSection.creditIntegrity)),
    );
  });

  test('path parsing fails closed for an unknown deep-link section', () {
    expect(
      AdminSection.fromPathSegment('credit-integrity'),
      AdminSection.creditIntegrity,
    );
    expect(AdminSection.fromPathSegment('future-admin'), isNull);
  });
}
