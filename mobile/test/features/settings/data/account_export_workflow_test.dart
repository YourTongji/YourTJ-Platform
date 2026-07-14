import 'dart:async';

import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/account/data/account_repository.dart';
import 'package:yourtj_mobile/features/settings/data/account_export_file_saver.dart';
import 'package:yourtj_mobile/features/settings/data/account_export_workflow.dart';

void main() {
  late AccountExportOwner? currentOwner;
  late StreamController<AccountExportOwner?> ownerChanges;
  late _ExportRepository repository;
  late _ExportSaver saver;
  late AccountExportWorkflow workflow;

  setUp(() {
    currentOwner = _ownerA;
    ownerChanges = StreamController<AccountExportOwner?>.broadcast(sync: true);
    repository = _ExportRepository();
    saver = _ExportSaver();
    workflow = AccountExportWorkflow(
      repository: repository,
      fileSaver: saver,
      ownerReader: () => currentOwner,
      ownerChanges: ownerChanges.stream,
    );
  });

  tearDown(() => ownerChanges.close());

  test(
    'a job loaded for account A never requests a grant as account B',
    () async {
      final AccountExportJobs jobs = await workflow.loadJobs();
      currentOwner = _ownerB;
      ownerChanges.add(_ownerB);

      await _expectSessionChange(
        workflow.saveJob(owner: jobs.owner, jobId: jobs.jobs.single.id),
      );

      expect(repository.grantCalls, 0);
      expect(repository.downloadCalls, 0);
      expect(saver.saveCalls, 0);
    },
  );

  test(
    'switching accounts while a grant is pending stops before download',
    () async {
      final AccountExportJobs jobs = await workflow.loadJobs();
      repository.pendingGrant = Completer<DataExportDownloadGrant>();
      final Future<AccountExportSaveResult> saving = workflow.saveJob(
        owner: jobs.owner,
        jobId: jobs.jobs.single.id,
      );
      await _waitFor(() => repository.grantCalls == 1);

      currentOwner = _ownerB;
      ownerChanges.add(_ownerB);
      repository.pendingGrant!.complete(_grant);

      await _expectSessionChange(saving);
      expect(repository.downloadCalls, 0);
      expect(saver.saveCalls, 0);
    },
  );

  test(
    'switching accounts while download is pending stops before native save',
    () async {
      final AccountExportJobs jobs = await workflow.loadJobs();
      repository.pendingDownload = Completer<AccountDataExport>();
      final Future<AccountExportSaveResult> saving = workflow.saveJob(
        owner: jobs.owner,
        jobId: jobs.jobs.single.id,
      );
      await _waitFor(() => repository.downloadCalls == 1);

      currentOwner = _ownerB;
      ownerChanges.add(_ownerB);
      repository.pendingDownload!.complete(_export);

      await _expectSessionChange(saving);
      expect(saver.saveCalls, 0);
    },
  );

  test(
    'switching accounts while the native picker is open cancels its save',
    () async {
      final AccountExportJobs jobs = await workflow.loadJobs();
      saver.pendingResult = Completer<AccountExportSaveResult>();
      final Future<AccountExportSaveResult> saving = workflow.saveJob(
        owner: jobs.owner,
        jobId: jobs.jobs.single.id,
      );
      await _waitFor(() => saver.saveCalls == 1);

      currentOwner = _ownerB;
      ownerChanges.add(_ownerB);

      await _expectSessionChange(saving);
      expect(saver.cancelCalls, 1);
      expect(saver.didPersist, isFalse);
    },
  );

  test(
    'switching accounts while create is pending suppresses old-account success',
    () async {
      repository.pendingCreate = Completer<DataExportJob>();
      final Future<DataExportJob> creating = workflow.createJob(
        owner: _ownerA,
        idempotencyKey: 'same-owner-retry-key',
      );
      await _waitFor(() => repository.createCalls == 1);

      currentOwner = _ownerB;
      ownerChanges.add(_ownerB);
      repository.pendingCreate!.complete(_job);

      await _expectSessionChange(creating);
    },
  );
}

const AccountExportOwner _ownerA = AccountExportOwner(
  accountId: 'account-a',
  sessionGeneration: 1,
);
const AccountExportOwner _ownerB = AccountExportOwner(
  accountId: 'account-b',
  sessionGeneration: 2,
);

final DataExportJob _job = DataExportJob(
  id: 'export-a',
  status: DataExportStatus.ready,
  createdAt: 100,
  updatedAt: 100,
  expiresAt: 200,
  errorCode: null,
);
final DataExportDownloadGrant _grant = DataExportDownloadGrant(
  token: 'a' * 43,
  expiresAt: 200,
);
final AccountDataExport _export = AccountDataExport(
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
  identity: const <String, Object?>{},
  forum: const <String, Object?>{},
  reviews: const <String, Object?>{},
  governance: const <String, Object?>{},
  credit: const <String, Object?>{},
  activity: const <String, Object?>{},
  platform: const <String, Object?>{},
  media: const <Object>[],
);

Future<void> _expectSessionChange(Future<Object?> operation) async {
  await expectLater(
    operation,
    throwsA(
      isA<AccountExportSaveFailure>().having(
        (AccountExportSaveFailure failure) => failure.kind,
        'kind',
        AccountExportSaveFailureKind.sessionChanged,
      ),
    ),
  );
}

Future<void> _waitFor(bool Function() predicate) async {
  for (int i = 0; i < 20; i += 1) {
    if (predicate()) {
      return;
    }
    await Future<void>.delayed(Duration.zero);
  }
  fail('asynchronous boundary was not reached');
}

class _ExportRepository extends AccountRepository {
  _ExportRepository()
    : super(
        IdentityApi(Dio()),
        AuthApi(Dio()),
        NotificationsApi(Dio()),
        MediaApi(Dio()),
      );

  int createCalls = 0;
  int grantCalls = 0;
  int downloadCalls = 0;
  Completer<DataExportJob>? pendingCreate;
  Completer<DataExportDownloadGrant>? pendingGrant;
  Completer<AccountDataExport>? pendingDownload;

  @override
  Future<List<DataExportJob>> getDataExports() async => <DataExportJob>[_job];

  @override
  Future<DataExportJob> createDataExport(String idempotencyKey) {
    createCalls += 1;
    return pendingCreate?.future ?? Future<DataExportJob>.value(_job);
  }

  @override
  Future<DataExportDownloadGrant> createDataExportGrant(String id) {
    grantCalls += 1;
    return pendingGrant?.future ??
        Future<DataExportDownloadGrant>.value(_grant);
  }

  @override
  Future<AccountDataExport> downloadDataExport({
    required String id,
    required String grant,
  }) {
    downloadCalls += 1;
    return pendingDownload?.future ?? Future<AccountDataExport>.value(_export);
  }
}

class _ExportSaver implements AccountExportFileSaver {
  int saveCalls = 0;
  int cancelCalls = 0;
  bool didPersist = false;
  Completer<AccountExportSaveResult>? pendingResult;

  @override
  Future<AccountExportSaveResult> save(AccountDataExport export) async {
    saveCalls += 1;
    final AccountExportSaveResult result =
        await (pendingResult?.future ??
            Future<AccountExportSaveResult>.value(
              AccountExportSaveResult.saved,
            ));
    if (result == AccountExportSaveResult.saved) {
      didPersist = true;
    }
    return result;
  }

  @override
  Future<void> cancelPendingSave() async {
    cancelCalls += 1;
    final Completer<AccountExportSaveResult>? pending = pendingResult;
    if (pending != null && !pending.isCompleted) {
      pending.complete(AccountExportSaveResult.cancelled);
    }
  }
}
