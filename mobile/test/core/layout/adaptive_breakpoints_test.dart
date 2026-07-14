import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/layout/adaptive_breakpoints.dart';

void main() {
  group('AdaptiveBreakpoints', () {
    test('uses compact navigation below 600 logical pixels', () {
      expect(AdaptiveBreakpoints.classify(320), WindowSizeClass.compact);
      expect(AdaptiveBreakpoints.classify(599.9), WindowSizeClass.compact);
    });

    test('uses a navigation rail between 600 and 839 logical pixels', () {
      expect(AdaptiveBreakpoints.classify(600), WindowSizeClass.medium);
      expect(AdaptiveBreakpoints.classify(839.9), WindowSizeClass.medium);
    });

    test('uses the expandable layout from 840 logical pixels', () {
      expect(AdaptiveBreakpoints.classify(840), WindowSizeClass.expanded);
    });
  });
}
