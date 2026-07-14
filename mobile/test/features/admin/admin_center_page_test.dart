import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/features/account/presentation/account_page.dart';
import 'package:yourtj_mobile/features/admin/presentation/admin_center_page.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';

void main() {
  testWidgets('deep link is denied when its capability was not issued', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        account: _account(<String>['users.search']),
        child: const AdminCenterPage(requestedSectionPath: 'credit-integrity'),
      ),
    );
    await tester.pump();

    expect(find.text('无权访问此模块'), findsOneWidget);
    expect(find.textContaining('capabilities'), findsOneWidget);
    expect(find.text('积分完整性'), findsNothing);
  });

  testWidgets('unknown deep link fails closed instead of opening overview', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        account: _account(<String>['users.search']),
        child: const AdminCenterPage(requestedSectionPath: 'not-a-module'),
      ),
    );
    await tester.pump();

    expect(find.text('管理模块不存在'), findsOneWidget);
    expect(find.text('概览'), findsNothing);
  });

  testWidgets('module cards expose only exact server-issued capabilities', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        account: _account(<String>['appeals.review']),
        child: const AdminCenterPage(),
      ),
    );
    await tester.pump();

    expect(find.text('申诉'), findsOneWidget);
    expect(find.text('用户'), findsNothing);
    expect(find.text('审核'), findsNothing);
    expect(find.textContaining('不会自动重试'), findsOneWidget);
  });

  testWidgets('account entry ignores role and follows capabilities only', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      _app(
        account: _account(const <String>[], role: AccountRoleEnum.admin),
        child: const AccountPage(),
      ),
    );
    await tester.pump();
    expect(find.text('管理中心'), findsNothing);

    await tester.pumpWidget(
      _app(
        account: _account(<String>['audit.read']),
        child: const AccountPage(),
      ),
    );
    await tester.pump();
    expect(find.text('管理中心'), findsOneWidget);
  });
}

Widget _app({required Account account, required Widget child}) {
  return ProviderScope(
    key: ValueKey<String>(
      '${account.role}:${account.capabilities.join(',')}:${child.runtimeType}',
    ),
    overrides: [
      sessionStateProvider.overrideWith(
        (Ref ref) => Stream<SessionState>.value(
          SessionState.authenticated(generation: 1, account: account),
        ),
      ),
    ],
    child: MaterialApp(home: child),
  );
}

Account _account(
  List<String> capabilities, {
  AccountRoleEnum role = AccountRoleEnum.user,
}) {
  return Account(
    id: 'account-1',
    handle: 'staff',
    avatarUrl: null,
    role: role,
    capabilities: capabilities,
    trustLevel: 2,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 1,
  );
}
