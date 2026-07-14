import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/core/design/app_theme.dart';
import 'package:yourtj_mobile/features/account/data/account_repository.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/settings/data/account_export_file_saver.dart';
import 'package:yourtj_mobile/features/settings/data/account_export_workflow.dart';
import 'package:yourtj_mobile/features/settings/presentation/data_exports_page.dart';

void main() {
  testWidgets('account change clears old jobs before loading the new owner', (
    WidgetTester tester,
  ) async {
    final StreamController<SessionState> sessions =
        StreamController<SessionState>();
    final StreamController<AccountExportOwner?> owners =
        StreamController<AccountExportOwner?>.broadcast(sync: true);
    addTearDown(sessions.close);
    addTearDown(owners.close);
    AccountExportOwner? currentOwner = _ownerA;
    final _PageRepository repository = _PageRepository('owner-a-marker');
    final _PageSaver saver = _PageSaver();
    final AccountExportWorkflow workflow = AccountExportWorkflow(
      repository: repository,
      fileSaver: saver,
      ownerReader: () => currentOwner,
      ownerChanges: owners.stream,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          accountRepositoryProvider.overrideWithValue(repository),
          accountExportWorkflowProvider.overrideWithValue(workflow),
          sessionStateProvider.overrideWith((Ref ref) => sessions.stream),
        ],
        child: MaterialApp(
          theme: AppTheme.light,
          home: const DataExportsPage(),
        ),
      ),
    );
    sessions.add(
      SessionState.authenticated(generation: 1, account: _account('account-a')),
    );
    await tester.pumpAndSettle();
    expect(find.textContaining('owner-a-marker'), findsOneWidget);

    repository.marker = 'owner-b-marker';
    currentOwner = _ownerB;
    owners.add(_ownerB);
    sessions.add(
      SessionState.authenticated(generation: 2, account: _account('account-b')),
    );
    await tester.pump();
    await tester.pump();
    expect(find.textContaining('owner-a-marker'), findsNothing);
    await tester.pumpAndSettle();
    expect(find.textContaining('owner-b-marker'), findsOneWidget);
  });
}

const AccountExportOwner _ownerA = AccountExportOwner(
  accountId: 'account-a',
  sessionGeneration: 1,
);
const AccountExportOwner _ownerB = AccountExportOwner(
  accountId: 'account-b',
  sessionGeneration: 2,
);

Account _account(String id) {
  return Account(
    id: id,
    handle: id,
    avatarUrl: null,
    role: AccountRoleEnum.user,
    capabilities: const <String>[],
    trustLevel: 1,
    hasPassword: true,
    onboardingRequired: false,
    createdAt: 100,
  );
}

class _PageRepository extends AccountRepository {
  _PageRepository(this.marker)
    : super(
        IdentityApi(Dio()),
        AuthApi(Dio()),
        NotificationsApi(Dio()),
        MediaApi(Dio()),
      );

  String marker;

  @override
  Future<List<DataExportJob>> getDataExports() async => <DataExportJob>[
    DataExportJob(
      id: 'export-id',
      status: DataExportStatus.ready,
      createdAt: 100,
      updatedAt: 100,
      expiresAt: 200,
      errorCode: marker,
    ),
  ];

  @override
  Future<RecentAuthStatus> getRecentAuthStatus() async => RecentAuthStatus(
    sessionBound: true,
    isFresh: true,
    authenticatedAt: 100,
    expiresAt: 200,
    method: RecentAuthMethod.password,
    availableMethods: const <RecentAuthMethod>[RecentAuthMethod.password],
  );

  @override
  Future<DataExportDownloadGrant> createDataExportGrant(String id) async =>
      DataExportDownloadGrant(token: 'a' * 43, expiresAt: 200);

  @override
  Future<AccountDataExport> downloadDataExport({
    required String id,
    required String grant,
  }) async => AccountDataExport(
    schemaVersion: 'yourtj.account-export.v2',
    generatedAt: 100,
    includedSections: const <String>[
      'identity',
      'forum',
      'reviews',
      'governance',
      'credit',
      'activity',
      'platform',
      'mediaMetadata',
    ],
    identity: const <String, Object?>{'value': 'private-export-value'},
    forum: const <String, Object?>{},
    reviews: const <String, Object?>{},
    governance: const <String, Object?>{},
    credit: const <String, Object?>{},
    activity: const <String, Object?>{},
    platform: const <String, Object?>{},
    media: const <Object>[],
  );
}

class _PageSaver implements AccountExportFileSaver {
  @override
  Future<AccountExportSaveResult> save(AccountDataExport export) async {
    return AccountExportSaveResult.saved;
  }

  @override
  Future<void> cancelPendingSave() async {}
}
