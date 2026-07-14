import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../account/data/account_repository.dart';
import '../../auth/data/session_manager.dart';
import '../../auth/domain/session_state.dart';
import 'account_export_file_saver.dart';

final Provider<AccountExportWorkflow> accountExportWorkflowProvider =
    Provider<AccountExportWorkflow>((Ref ref) {
      final SessionManager session = ref.watch(sessionManagerProvider);
      return AccountExportWorkflow(
        repository: ref.watch(accountRepositoryProvider),
        fileSaver: ref.watch(accountExportFileSaverProvider),
        ownerReader: () => _ownerForState(session.state),
        ownerChanges: session.changes.map(_ownerForState),
      );
    });

class AccountExportOwner {
  const AccountExportOwner({
    required this.accountId,
    required this.sessionGeneration,
  });

  final String accountId;
  final int sessionGeneration;
}

class AccountExportJobs {
  AccountExportJobs({required this.owner, required List<DataExportJob> jobs})
    : jobs = List<DataExportJob>.unmodifiable(jobs);

  final AccountExportOwner owner;
  final List<DataExportJob> jobs;
}

typedef AccountExportOwnerReader = AccountExportOwner? Function();

class AccountExportWorkflow {
  factory AccountExportWorkflow({
    required AccountRepository repository,
    required AccountExportFileSaver fileSaver,
    required AccountExportOwnerReader ownerReader,
    required Stream<AccountExportOwner?> ownerChanges,
  }) {
    return AccountExportWorkflow._(
      repository,
      fileSaver,
      ownerReader,
      ownerChanges,
    );
  }

  AccountExportWorkflow._(
    this._repository,
    this._fileSaver,
    this._ownerReader,
    this._ownerChanges,
  );

  final AccountRepository _repository;
  final AccountExportFileSaver _fileSaver;
  final AccountExportOwnerReader _ownerReader;
  final Stream<AccountExportOwner?> _ownerChanges;

  AccountExportOwner captureOwner() {
    final AccountExportOwner? owner = _ownerReader();
    if (owner == null) {
      throw _sessionChangedFailure();
    }
    return owner;
  }

  void ensureCurrentOwner(AccountExportOwner expected) {
    if (!_isCurrentOwner(expected)) {
      throw _sessionChangedFailure();
    }
  }

  Future<AccountExportJobs> loadJobs() async {
    final AccountExportOwner owner = captureOwner();
    final List<DataExportJob> jobs = await _runForOwner(
      owner,
      _repository.getDataExports,
    );
    return AccountExportJobs(owner: owner, jobs: jobs);
  }

  Future<DataExportJob> createJob({
    required AccountExportOwner owner,
    required String idempotencyKey,
  }) {
    return _runForOwner(
      owner,
      () => _repository.createDataExport(idempotencyKey),
    );
  }

  Future<AccountExportSaveResult> saveJob({
    required AccountExportOwner owner,
    required String jobId,
  }) async {
    final DataExportDownloadGrant grant = await _runForOwner(
      owner,
      () => _repository.createDataExportGrant(jobId),
    );
    final AccountDataExport export = await _runForOwner(
      owner,
      () => _repository.downloadDataExport(id: jobId, grant: grant.token),
    );

    Future<void>? cancellation;
    final StreamSubscription<AccountExportOwner?> subscription = _ownerChanges
        .listen((AccountExportOwner? current) {
          if (!_matches(owner, current)) {
            final Future<void> pendingCancellation = cancellation ??=
                _cancelPendingSave();
            unawaited(pendingCancellation);
          }
        });
    try {
      ensureCurrentOwner(owner);
      return await _runForOwner(owner, () => _fileSaver.save(export));
    } finally {
      await subscription.cancel();
      final Future<void>? pendingCancellation = cancellation;
      if (pendingCancellation != null) {
        await pendingCancellation;
      }
    }
  }

  Future<T> _runForOwner<T>(
    AccountExportOwner owner,
    Future<T> Function() operation,
  ) async {
    ensureCurrentOwner(owner);
    try {
      final T value = await operation();
      ensureCurrentOwner(owner);
      return value;
    } on Object {
      ensureCurrentOwner(owner);
      rethrow;
    }
  }

  Future<void> _cancelPendingSave() async {
    try {
      await _fileSaver.cancelPendingSave();
    } on Object {
      // The post-save owner check still suppresses success if the platform stops responding.
    }
  }

  bool _isCurrentOwner(AccountExportOwner expected) {
    return _matches(expected, _ownerReader());
  }
}

AccountExportOwner? _ownerForState(SessionState state) {
  final String? accountId = state.account?.id;
  if (!state.isAuthenticated || accountId == null) {
    return null;
  }
  return AccountExportOwner(
    accountId: accountId,
    sessionGeneration: state.generation,
  );
}

bool _matches(AccountExportOwner expected, AccountExportOwner? current) {
  return current != null &&
      current.accountId == expected.accountId &&
      current.sessionGeneration == expected.sessionGeneration;
}

AccountExportSaveFailure _sessionChangedFailure() {
  return const AccountExportSaveFailure(
    kind: AccountExportSaveFailureKind.sessionChanged,
    message: '账号或登录状态已变化，已停止旧账号的数据导出保存。请刷新后重新操作。',
  );
}
