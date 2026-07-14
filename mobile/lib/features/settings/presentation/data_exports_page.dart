import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:uuid/uuid.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/presentation/account_page_layout.dart';
import '../../auth/domain/session_state.dart';
import '../data/account_export_file_saver.dart';
import '../data/account_export_workflow.dart';
import 'recent_auth_dialog.dart';

class DataExportsPage extends ConsumerStatefulWidget {
  const DataExportsPage({super.key});

  @override
  ConsumerState<DataExportsPage> createState() => _DataExportsPageState();
}

class _DataExportsPageState extends ConsumerState<DataExportsPage> {
  AccountExportJobs? _jobs;
  ApiFailure? _failure;
  String? _saveFailureMessage;
  bool _isLoading = true;
  bool _isCreating = false;
  String? _downloadingId;
  String? _pendingCreateIdempotencyKey;
  String? _sessionAccountId;
  int? _sessionGeneration;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final int loadGeneration = ++_loadGeneration;
    setState(() {
      _isLoading = true;
      _failure = null;
      _saveFailureMessage = null;
    });
    try {
      final AccountExportJobs jobs = await ref
          .read(accountExportWorkflowProvider)
          .loadJobs();
      if (mounted && loadGeneration == _loadGeneration) {
        setState(() => _jobs = jobs);
      }
    } on AccountExportSaveFailure catch (failure) {
      if (mounted && loadGeneration == _loadGeneration) {
        setState(() => _saveFailureMessage = failure.message);
      }
    } on ApiFailure catch (failure) {
      if (mounted && loadGeneration == _loadGeneration) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted && loadGeneration == _loadGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _create() async {
    if (_isCreating) {
      return;
    }
    final AccountExportWorkflow workflow = ref.read(
      accountExportWorkflowProvider,
    );
    try {
      final AccountExportOwner owner = workflow.captureOwner();
      workflow.ensureCurrentOwner(owner);
      final bool verified = await ensureRecentAuthentication(context, ref);
      if (!verified || !mounted) {
        return;
      }
      workflow.ensureCurrentOwner(owner);
      setState(() {
        _isCreating = true;
        _failure = null;
        _saveFailureMessage = null;
        _pendingCreateIdempotencyKey ??= const Uuid().v4();
      });
      await workflow.createJob(
        owner: owner,
        idempotencyKey: _pendingCreateIdempotencyKey!,
      );
      _pendingCreateIdempotencyKey = null;
      await _load();
      workflow.ensureCurrentOwner(owner);
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(const SnackBar(content: Text('导出任务已创建或返回现有任务')));
      }
    } on AccountExportSaveFailure catch (failure) {
      if (mounted) {
        setState(() => _saveFailureMessage = failure.message);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isCreating = false);
      }
    }
  }

  Future<void> _download(DataExportJob job, AccountExportOwner owner) async {
    if (_downloadingId != null) {
      return;
    }
    final AccountExportWorkflow workflow = ref.read(
      accountExportWorkflowProvider,
    );
    try {
      workflow.ensureCurrentOwner(owner);
      final bool verified = await ensureRecentAuthentication(context, ref);
      if (!verified || !mounted) {
        return;
      }
      workflow.ensureCurrentOwner(owner);
      setState(() {
        _downloadingId = job.id;
        _failure = null;
        _saveFailureMessage = null;
      });
      final AccountExportSaveResult saveResult = await workflow.saveJob(
        owner: owner,
        jobId: job.id,
      );
      if (!mounted) {
        return;
      }
      final String message = switch (saveResult) {
        AccountExportSaveResult.saved => '已保存到你选择的位置',
        AccountExportSaveResult.cancelled => '已取消保存，未写入文件',
      };
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(message)));
    } on AccountExportSaveFailure catch (failure) {
      if (mounted) {
        setState(() => _saveFailureMessage = failure.message);
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _downloadingId = null);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final SessionState? session = ref.watch(sessionStateProvider).value;
    final String? accountId = session?.account?.id;
    if (session != null &&
        (_sessionAccountId != accountId ||
            _sessionGeneration != session.generation)) {
      _sessionAccountId = accountId;
      _sessionGeneration = session.generation;
      ++_loadGeneration;
      _jobs = null;
      _failure = null;
      _saveFailureMessage = null;
      _downloadingId = null;
      _pendingCreateIdempotencyKey = null;
      _isCreating = false;
      _isLoading = session.isAuthenticated;
      final int expectedGeneration = session.generation;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        final SessionState? current = ref.read(sessionStateProvider).value;
        if (!mounted ||
            current?.account?.id != accountId ||
            current?.generation != expectedGeneration) {
          return;
        }
        if (current!.isAuthenticated) {
          unawaited(_load());
        }
      });
    }
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取数据导出任务',
        description: '任务由各数据拥有域组合，成果只保留有限时间。',
      );
    } else if (_jobs == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildJobs(_jobs);
    }
    return AccountPageLayout(title: '我的数据导出', child: child);
  }

  Widget _buildJobs(AccountExportJobs? snapshot) {
    final List<DataExportJob> jobs = snapshot?.jobs ?? <DataExportJob>[];
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        children: <Widget>[
          const Card(
            child: Padding(
              padding: EdgeInsets.all(16),
              child: Text(
                '创建任务和下载授权都需要最近认证。导出不包含他人发来的私信正文、举报人/审核人身份、内部证据或凭据秘密。下载后会打开系统文件选择器，只有你确认的位置会收到 JSON 文件；应用不会显示全文、写入普通缓存或复制到剪贴板。',
              ),
            ),
          ),
          const SizedBox(height: 12),
          FilledButton.icon(
            onPressed: _isCreating ? null : _create,
            icon: _isCreating
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.archive_outlined),
            label: Text(_isCreating ? '正在创建' : '创建新的导出任务'),
          ),
          if (jobs.isEmpty) ...<Widget>[
            const SizedBox(height: 20),
            const AppEmptyState(
              title: '还没有导出任务',
              description: '创建后可以下拉刷新查看后台组合进度。',
            ),
          ] else ...<Widget>[
            const SizedBox(height: 20),
            ...jobs.map((DataExportJob job) => _jobCard(job, snapshot!.owner)),
          ],
          if (_failure != null) ...<Widget>[
            const SizedBox(height: 16),
            AppErrorState(
              title: '导出操作失败',
              description: _failure!.message,
              onRetry: _load,
            ),
          ],
          if (_saveFailureMessage != null) ...<Widget>[
            const SizedBox(height: 16),
            AppErrorState(
              title: '导出文件未确认保存',
              description: _saveFailureMessage!,
            ),
          ],
        ],
      ),
    );
  }

  Widget _jobCard(DataExportJob job, AccountExportOwner owner) {
    final bool canDownload = job.status == DataExportStatus.ready;
    final bool isDownloading = _downloadingId == job.id;
    return Card(
      child: ListTile(
        leading: Icon(_iconForStatus(job.status)),
        title: Text(_labelForStatus(job.status)),
        subtitle: Text(
          '创建 ${formatAccountTime(job.createdAt)}\n'
          '成果到期 ${formatAccountTime(job.expiresAt)}'
          '${job.errorCode == null ? '' : '\n错误代码 ${job.errorCode}'}',
        ),
        isThreeLine: true,
        trailing: canDownload
            ? IconButton(
                tooltip: '最近认证后保存导出 JSON',
                onPressed: isDownloading ? null : () => _download(job, owner),
                icon: isDownloading
                    ? const CircularProgressIndicator()
                    : const Icon(Icons.download_outlined),
              )
            : null,
      ),
    );
  }
}

String _labelForStatus(DataExportStatus status) {
  return switch (status) {
    DataExportStatus.queued => '已排队',
    DataExportStatus.running => '正在组合',
    DataExportStatus.ready => '可下载',
    DataExportStatus.failed => '组合失败',
    DataExportStatus.expired => '已过期',
    DataExportStatus.unknownDefaultOpenApi => '未知状态',
  };
}

IconData _iconForStatus(DataExportStatus status) {
  return switch (status) {
    DataExportStatus.queued => Icons.schedule_rounded,
    DataExportStatus.running => Icons.sync_rounded,
    DataExportStatus.ready => Icons.download_done_rounded,
    DataExportStatus.failed => Icons.error_outline_rounded,
    DataExportStatus.expired => Icons.timer_off_outlined,
    DataExportStatus.unknownDefaultOpenApi => Icons.help_outline_rounded,
  };
}
