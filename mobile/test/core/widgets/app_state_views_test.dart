import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart' show SemanticsNode;
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/core/l10n/app_strings.dart';
import 'package:yourtj_mobile/core/widgets/app_state_views.dart';

void main() {
  testWidgets('loading state announces progress', (WidgetTester tester) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    try {
      await _pumpState(tester, const AppLoadingState());

      final SemanticsNode announcement = tester.getSemantics(
        find.byKey(const Key('app-state-announcement')),
      );

      expect(
        announcement.getSemanticsData().label,
        '${AppStrings.loadingTitle}，${AppStrings.loadingDescription}',
      );
      expect(
        announcement.getSemanticsData().flagsCollection.isLiveRegion,
        isTrue,
      );
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets('empty state exposes its action', (WidgetTester tester) async {
    var wasPressed = false;
    await _pumpState(
      tester,
      AppEmptyState(
        action: FilledButton(
          onPressed: () => wasPressed = true,
          child: const Text(AppStrings.retry),
        ),
      ),
    );

    await tester.tap(find.text(AppStrings.retry));

    expect(wasPressed, isTrue);
  });

  testWidgets('error state offers an accessible retry', (
    WidgetTester tester,
  ) async {
    var retryCount = 0;
    await _pumpState(tester, AppErrorState(onRetry: () => retryCount += 1));

    await tester.tap(find.text(AppStrings.retry));

    expect(retryCount, 1);
    expect(find.byIcon(Icons.refresh_rounded), findsOneWidget);
  });

  testWidgets('permission state explains the denial', (
    WidgetTester tester,
  ) async {
    await _pumpState(tester, const AppPermissionState());

    expect(find.text(AppStrings.permissionTitle), findsOneWidget);
    expect(find.text(AppStrings.permissionDescription), findsOneWidget);
  });
}

Future<void> _pumpState(WidgetTester tester, Widget state) async {
  await tester.pumpWidget(
    MaterialApp(
      theme: AppTheme.light,
      home: Scaffold(body: state),
    ),
  );
}
