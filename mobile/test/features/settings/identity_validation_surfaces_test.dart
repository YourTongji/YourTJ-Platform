import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/account/data/account_repository.dart';
import 'package:yourtj_mobile/features/settings/presentation/recent_auth_dialog.dart';
import 'package:yourtj_mobile/features/settings/presentation/recovery_page.dart';

void main() {
  testWidgets('recovery form requires an exact six-digit email code', (
    WidgetTester tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(theme: AppTheme.light, home: const RecoveryPage()),
      ),
    );

    await tester.tap(find.text('邮箱验证码'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byType(TextFormField).at(0),
      'student@tongji.edu.cn',
    );
    await tester.enterText(find.byType(TextFormField).at(1), '12345a');
    await tester.tap(find.text('检查是否可恢复'));
    await tester.pump();
    expect(find.text('请输入 6 位数字验证码'), findsOneWidget);

    await tester.enterText(find.byType(TextFormField).at(1), '123456');
    await tester.tap(find.text('检查是否可恢复'));
    await tester.pump();
    expect(find.text('请输入 6 位数字验证码'), findsNothing);
    expect(find.text('请先完成验证码发送流程'), findsOneWidget);
  });

  testWidgets('recent authentication rejects non-six-digit email codes', (
    WidgetTester tester,
  ) async {
    final _RecentAuthRepository repository = _RecentAuthRepository();
    bool? result;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [accountRepositoryProvider.overrideWithValue(repository)],
        child: MaterialApp(
          theme: AppTheme.light,
          home: Consumer(
            builder: (BuildContext context, WidgetRef ref, Widget? child) {
              return Scaffold(
                body: FilledButton(
                  onPressed: () async {
                    result = await ensureRecentAuthentication(context, ref);
                  },
                  child: const Text('验证'),
                ),
              );
            },
          ),
        ),
      ),
    );

    await tester.tap(find.text('验证'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextFormField), '12345a');
    await tester.tap(find.text('确认'));
    await tester.pump();
    expect(find.text('请输入 6 位数字验证码'), findsOneWidget);

    await tester.enterText(find.byType(TextFormField), '123456');
    await tester.tap(find.text('确认'));
    await tester.pumpAndSettle();

    expect(repository.verifiedCode, '123456');
    expect(result, isTrue);
  });
}

class _RecentAuthRepository extends AccountRepository {
  _RecentAuthRepository()
    : super(
        IdentityApi(Dio()),
        AuthApi(Dio()),
        NotificationsApi(Dio()),
        MediaApi(Dio()),
      );

  String? verifiedCode;

  @override
  Future<RecentAuthStatus> getRecentAuthStatus() async =>
      _status(isFresh: false);

  @override
  Future<RecentAuthStatus> verifyRecentAuth(RecentAuthVerifyInput input) async {
    verifiedCode = input.code;
    return _status(isFresh: true);
  }

  RecentAuthStatus _status({required bool isFresh}) {
    return RecentAuthStatus(
      sessionBound: true,
      isFresh: isFresh,
      authenticatedAt: isFresh ? 100 : null,
      expiresAt: isFresh ? 700 : null,
      method: isFresh ? RecentAuthMethod.emailCode : null,
      availableMethods: const <RecentAuthMethod>[RecentAuthMethod.emailCode],
    );
  }
}
