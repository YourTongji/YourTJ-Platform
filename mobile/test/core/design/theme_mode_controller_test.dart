import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/core/design/theme_mode_controller.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/settings/presentation/settings_page.dart';

void main() {
  test('restores and persists an explicit theme choice', () async {
    final _FakeThemeModePreferences preferences = _FakeThemeModePreferences(
      'dark',
    );
    final ThemeModeController controller = ThemeModeController(
      preferences: preferences,
    );

    await controller.restore();
    expect(controller.mode, ThemeMode.dark);

    await controller.setMode(ThemeMode.light);
    expect(controller.mode, ThemeMode.light);
    expect(preferences.value, 'light');
  });

  test('unknown and unavailable preferences fail safely to system', () async {
    final ThemeModeController unknownController = ThemeModeController(
      preferences: _FakeThemeModePreferences('sepia'),
      initialMode: ThemeMode.dark,
    );
    await unknownController.restore();
    expect(unknownController.mode, ThemeMode.system);

    final ThemeModeController failingController = ThemeModeController(
      preferences: _FailingThemeModePreferences(),
    );
    await failingController.restore();
    expect(failingController.mode, ThemeMode.system);
    expect(
      failingController.persistenceFailure,
      ThemeModePersistenceFailure.load,
    );
    await failingController.setMode(ThemeMode.dark);
    expect(failingController.mode, ThemeMode.dark);
    expect(
      failingController.persistenceFailure,
      ThemeModePersistenceFailure.save,
    );
  });

  test('a delayed restore cannot replace a newer explicit selection', () async {
    final _DelayedReadThemeModePreferences preferences =
        _DelayedReadThemeModePreferences();
    final ThemeModeController controller = ThemeModeController(
      preferences: preferences,
    );

    final Future<void> restore = controller.restore();
    final Future<void> selection = controller.setMode(ThemeMode.dark);
    preferences.readResult.complete('light');
    await Future.wait<void>(<Future<void>>[restore, selection]);

    expect(controller.mode, ThemeMode.dark);
    expect(preferences.value, 'dark');
    expect(controller.persistenceFailure, isNull);
  });

  test(
    'rapid selections persist serially with the latest value last',
    () async {
      final _ControlledWriteThemeModePreferences preferences =
          _ControlledWriteThemeModePreferences();
      final ThemeModeController controller = ThemeModeController(
        preferences: preferences,
      );

      final Future<void> first = controller.setMode(ThemeMode.light);
      final Future<void> second = controller.setMode(ThemeMode.dark);
      await Future<void>.delayed(Duration.zero);

      expect(preferences.writes, <String>['light']);
      preferences.pendingWrites.first.complete();
      await Future<void>.delayed(Duration.zero);
      expect(preferences.writes, <String>['light', 'dark']);
      preferences.pendingWrites.last.complete();
      await Future.wait<void>(<Future<void>>[first, second]);

      expect(controller.mode, ThemeMode.dark);
      expect(preferences.value, 'dark');
      expect(controller.persistenceFailure, isNull);
    },
  );

  testWidgets('save failure is visible and can be retried', (
    WidgetTester tester,
  ) async {
    final _RetryingThemeModePreferences preferences =
        _RetryingThemeModePreferences();
    final ThemeModeController controller = ThemeModeController(
      preferences: preferences,
    );
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          sessionStateProvider.overrideWithValue(
            const AsyncValue<SessionState>.data(
              SessionState.anonymous(generation: 1),
            ),
          ),
        ],
        child: ThemeModeScope(
          controller: controller,
          child: const MaterialApp(home: SettingsPage()),
        ),
      ),
    );

    await tester.tap(find.widgetWithText(ChoiceChip, '深色'));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('theme-persistence-error')), findsOneWidget);
    expect(find.textContaining('未能保存'), findsOneWidget);
    expect(find.text('重试保存'), findsOneWidget);

    preferences.shouldFail = false;
    await tester.tap(find.byKey(const Key('theme-persistence-retry')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('theme-persistence-error')), findsNothing);
    expect(preferences.value, 'dark');
    expect(controller.persistenceFailure, isNull);
  });
}

class _FakeThemeModePreferences implements ThemeModePreferences {
  _FakeThemeModePreferences(this.value);

  String? value;

  @override
  Future<String?> read() async => value;

  @override
  Future<void> write(String value) async {
    this.value = value;
  }
}

class _FailingThemeModePreferences implements ThemeModePreferences {
  @override
  Future<String?> read() => Future<String?>.error(StateError('unavailable'));

  @override
  Future<void> write(String value) =>
      Future<void>.error(StateError('unavailable'));
}

class _DelayedReadThemeModePreferences implements ThemeModePreferences {
  final Completer<String?> readResult = Completer<String?>();
  String? value;

  @override
  Future<String?> read() => readResult.future;

  @override
  Future<void> write(String value) async {
    this.value = value;
  }
}

class _ControlledWriteThemeModePreferences implements ThemeModePreferences {
  final List<String> writes = <String>[];
  final List<Completer<void>> pendingWrites = <Completer<void>>[];
  String? value;

  @override
  Future<String?> read() async => value;

  @override
  Future<void> write(String value) async {
    writes.add(value);
    final Completer<void> completion = Completer<void>();
    pendingWrites.add(completion);
    await completion.future;
    this.value = value;
  }
}

class _RetryingThemeModePreferences implements ThemeModePreferences {
  bool shouldFail = true;
  String? value;

  @override
  Future<String?> read() async => value;

  @override
  Future<void> write(String value) async {
    if (shouldFail) {
      throw StateError('unavailable');
    }
    this.value = value;
  }
}
