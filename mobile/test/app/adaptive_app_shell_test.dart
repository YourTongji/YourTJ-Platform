import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app.dart';
import 'package:yourtj_mobile/app/router.dart';
import 'package:yourtj_mobile/core/l10n/app_strings.dart';
import 'package:yourtj_mobile/core/navigation/app_route_visibility.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/home/presentation/home_page.dart';
import 'package:yourtj_mobile/features/messages/domain/message_badge_counts.dart';

import '../support/shell_test_scope.dart';

void main() {
  testWidgets('320 layout uses bottom navigation and changes tabs', (
    WidgetTester tester,
  ) async {
    final GoRouter router = await _pumpApp(tester, const Size(320, 720));

    expect(find.byKey(const Key('compact-navigation')), findsOneWidget);
    expect(find.byType(NavigationRail), findsNothing);
    expect(
      TickerMode.valuesOf(tester.element(find.byType(HomePage))).enabled,
      isTrue,
    );

    await tester.tap(find.text(AppStrings.forum));
    await tester.pumpAndSettle();

    expect(router.routeInformationProvider.value.uri.path, AppRoutes.forum);
    expect(find.text('热门'), findsWidgets);
    expect(
      TickerMode.valuesOf(
        tester.element(find.byType(HomePage, skipOffstage: false)),
      ).enabled,
      isFalse,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('covering root route marks the shell content hidden', (
    WidgetTester tester,
  ) async {
    final GoRouter router = await _pumpApp(tester, const Size(390, 844));
    Finder home = find.byType(HomePage, skipOffstage: false);

    expect(AppRouteVisibilityScope.isVisibleOf(tester.element(home)), isTrue);

    unawaited(router.push(AppRoutes.login));
    await tester.pumpAndSettle();
    home = find.byType(HomePage, skipOffstage: false);

    expect(AppRouteVisibilityScope.isVisibleOf(tester.element(home)), isFalse);
  });

  testWidgets('600 layout uses the compact navigation rail', (
    WidgetTester tester,
  ) async {
    await _pumpApp(tester, const Size(600, 720));

    final NavigationRail rail = tester.widget<NavigationRail>(
      find.byKey(const Key('medium-navigation-rail')),
    );

    expect(rail.extended, isFalse);
    expect(find.byType(NavigationBar), findsNothing);
  });

  testWidgets('840 layout enables extended primary navigation', (
    WidgetTester tester,
  ) async {
    await _pumpApp(tester, const Size(840, 900));

    final NavigationRail rail = tester.widget<NavigationRail>(
      find.byKey(const Key('expanded-navigation-rail')),
    );

    expect(rail.extended, isTrue);
    expect(find.byKey(const Key('expanded-search-action')), findsOneWidget);
    expect(find.byType(NavigationBar), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('header and primary navigation expose semantic labels', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    try {
      await _pumpApp(tester, const Size(320, 720));

      expect(find.bySemanticsLabel(AppStrings.openSearch), findsOneWidget);
      expect(find.bySemanticsLabel(AppStrings.openMessages), findsOneWidget);
      expect(find.bySemanticsLabel(AppStrings.openAccount), findsOneWidget);
      expect(find.bySemanticsLabel(AppStrings.mainNavigation), findsOneWidget);
    } finally {
      semantics.dispose();
    }
  });

  testWidgets('header exposes the combined notification and DM badge', (
    WidgetTester tester,
  ) async {
    final SemanticsHandle semantics = tester.ensureSemantics();
    try {
      await _pumpApp(
        tester,
        const Size(390, 844),
        messageBadges: const MessageBadgeCounts(
          notifications: 2,
          governance: 1,
          directMessages: 3,
          requests: 1,
        ),
      );

      expect(find.text('7'), findsOneWidget);
      expect(
        find.bySemanticsLabel('${AppStrings.openMessages}，7 条未读'),
        findsOneWidget,
      );
    } finally {
      semantics.dispose();
    }
  });

  testWidgets('authenticated avatar opens the aligned account destinations', (
    WidgetTester tester,
  ) async {
    await _pumpApp(
      tester,
      const Size(390, 844),
      sessionState: SessionState.authenticated(
        account: _account(),
        generation: 1,
      ),
    );

    await tester.tap(find.byKey(const Key('header-account-menu')));
    await tester.pumpAndSettle();

    expect(find.text('公告'), findsOneWidget);
    expect(find.text('收藏'), findsOneWidget);
    expect(find.text('申诉'), findsOneWidget);
    expect(find.text('设置'), findsOneWidget);
    expect(find.text('管理'), findsNothing);
  });
}

Future<GoRouter> _pumpApp(
  WidgetTester tester,
  Size size, {
  MessageBadgeCounts messageBadges = MessageBadgeCounts.zero,
  SessionState sessionState = const SessionState.anonymous(generation: 1),
}) async {
  tester.view.devicePixelRatio = 1;
  tester.view.physicalSize = size;
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(tester.view.resetPhysicalSize);
  final GoRouter router = createAppRouter();
  addTearDown(router.dispose);

  await tester.pumpWidget(
    shellTestScope(
      messageBadges: messageBadges,
      sessionState: sessionState,
      child: YourTjApp(router: router, enableAnnouncementGate: false),
    ),
  );
  await tester.pumpAndSettle();
  return router;
}

Account _account() {
  return Account(
    id: 'account-1',
    handle: 'student',
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: const <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 1,
  );
}
