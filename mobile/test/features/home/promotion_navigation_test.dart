import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/home/domain/promotion_navigation.dart';

void main() {
  test('accepts only supported same-origin application locations', () {
    expect(
      PromotionNavigation.internalLocation('/forum/threads/42?from=promotion'),
      '/forum/threads/42?from=promotion',
    );
    expect(
      PromotionNavigation.internalLocation('/courses/course-1'),
      '/courses/course-1',
    );
    expect(
      PromotionNavigation.internalLocation('/profile/alice'),
      '/profile/alice',
    );

    expect(
      PromotionNavigation.internalLocation('https://attacker.example/forum'),
      isNull,
    );
    expect(PromotionNavigation.internalLocation('//attacker.example'), isNull);
    expect(PromotionNavigation.internalLocation('/admin/users'), isNull);
    expect(PromotionNavigation.internalLocation('/forum#credential'), isNull);
    expect(PromotionNavigation.internalLocation(r'/forum\private'), isNull);
  });
}
