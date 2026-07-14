import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/app/app.dart';
import 'package:yourtj_mobile/app/router.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';

import '../../support/shell_test_scope.dart';

void main() {
  test('light palette matches the Web semantic tokens', () {
    final YourTjPalette palette = AppTheme.light.extension<YourTjPalette>()!;

    expect(palette.background, const Color(0xFFF8FAF8));
    expect(palette.foreground, const Color(0xFF191C1B));
    expect(palette.card, const Color(0xFFF2F4F2));
    expect(palette.primary, const Color(0xFF009688));
    expect(palette.secondary, const Color(0xFFF0FDFA));
    expect(palette.muted, const Color(0xFFECEEEC));
    expect(palette.destructive, const Color(0xFFD4183D));
    expect(palette.border, const Color(0xFFE1E3E1));
    expect(palette.chartColors.first, const Color(0xFF009688));
  });

  test('dark palette matches the Web semantic tokens', () {
    final YourTjPalette palette = AppTheme.dark.extension<YourTjPalette>()!;

    expect(palette.background, const Color(0xFF0C1E1B));
    expect(palette.foreground, const Color(0xFFD8EDEA));
    expect(palette.card, const Color(0xFF132922));
    expect(palette.primary, const Color(0xFF2ECFB2));
    expect(palette.secondary, const Color(0xFF1A3832));
    expect(palette.muted, const Color(0xFF1A3832));
    expect(palette.accent, const Color(0xFF1E4039));
    expect(palette.destructive, const Color(0xFFF04060));
    expect(palette.border, const Color(0x242ECFB2));
    expect(palette.chartColors.first, const Color(0xFF009688));
  });

  testWidgets('dark theme reaches the routed application', (
    WidgetTester tester,
  ) async {
    final router = createAppRouter();
    addTearDown(router.dispose);

    await tester.pumpWidget(
      shellTestScope(
        child: YourTjApp(
          router: router,
          themeMode: ThemeMode.dark,
          enableAnnouncementGate: false,
        ),
      ),
    );
    await tester.pumpAndSettle();

    final BuildContext scaffoldContext = tester.element(
      find.byType(Scaffold).first,
    );
    final ThemeData theme = Theme.of(scaffoldContext);

    expect(theme.brightness, Brightness.dark);
    expect(theme.scaffoldBackgroundColor, const Color(0xFF0C1E1B));
  });

  test('motion extension keeps the Web duration scale', () {
    final YourTjMotion motion = AppTheme.light.extension<YourTjMotion>()!;

    expect(motion.fast, const Duration(milliseconds: 120));
    expect(motion.normal, const Duration(milliseconds: 200));
    expect(motion.slow, const Duration(milliseconds: 320));
  });
}
