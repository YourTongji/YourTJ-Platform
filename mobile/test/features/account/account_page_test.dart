import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/app/router.dart';
import 'package:yourtj_mobile/features/account/presentation/account_page.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';

void main() {
  testWidgets('authenticated account exposes the appeals center', (
    WidgetTester tester,
  ) async {
    final GoRouter router = GoRouter(
      initialLocation: AppRoutes.account,
      routes: <RouteBase>[
        GoRoute(
          path: AppRoutes.account,
          builder: (BuildContext context, GoRouterState state) =>
              const AccountPage(),
        ),
        GoRoute(
          path: AppRoutes.appeals,
          builder: (BuildContext context, GoRouterState state) =>
              const Scaffold(body: Text('申诉目标页')),
        ),
      ],
    );
    addTearDown(router.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          sessionStateProvider.overrideWith(
            (Ref ref) => Stream<SessionState>.value(
              SessionState.authenticated(generation: 4, account: _account()),
            ),
          ),
        ],
        child: MaterialApp.router(routerConfig: router),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('申诉中心'), findsOneWidget);
    expect(find.text('查看本人治理事件、申诉进度与可用操作'), findsOneWidget);

    await tester.tap(find.text('申诉中心'));
    await tester.pumpAndSettle();

    expect(find.text('申诉目标页'), findsOneWidget);
  });
}

Account _account() {
  return Account(
    id: 'account-1',
    handle: 'student',
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 1,
  );
}
