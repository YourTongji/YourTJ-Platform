import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../data/announcements_repository.dart';
import '../data/anonymous_announcement_seen_store.dart';
import 'announcement_dialog.dart';

class AnnouncementsPage extends ConsumerStatefulWidget {
  const AnnouncementsPage({this.embedded = false, super.key});

  final bool embedded;

  @override
  ConsumerState<AnnouncementsPage> createState() => _AnnouncementsPageState();
}

class _AnnouncementsPageState extends ConsumerState<AnnouncementsPage>
    with AutomaticKeepAliveClientMixin {
  List<Announcement> _announcements = <Announcement>[];
  bool _isLoading = true;
  bool _isMutating = false;
  int _requestGeneration = 0;
  int? _sessionGeneration;
  ApiFailure? _failure;

  AnnouncementsRepository get _repository =>
      ref.read(announcementsRepositoryProvider);

  @override
  bool get wantKeepAlive => true;

  Future<void> _load() async {
    final int generation = ++_requestGeneration;
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final List<Announcement> announcements = await _repository.active();
      if (!mounted || generation != _requestGeneration) {
        return;
      }
      setState(() {
        _announcements = announcements;
      });
    } on ApiFailure catch (failure) {
      if (mounted && generation == _requestGeneration) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted && generation == _requestGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _recordSeen(
    Announcement announcement,
    bool authenticated,
  ) async {
    if (!authenticated) {
      await ref
          .read(anonymousAnnouncementSeenStoreProvider)
          .remember(announcement);
      return;
    }
    try {
      await _repository.record(
        announcement: announcement,
        action: AnnouncementReceiptInputActionEnum.seen,
      );
    } on ApiFailure catch (failure) {
      _showMessage('公告已显示，但查看状态暂未同步：${failure.message}');
    }
  }

  Future<void> _recordAction(
    Announcement announcement,
    AnnouncementReceiptInputActionEnum action,
  ) async {
    try {
      await _repository.record(announcement: announcement, action: action);
    } on ApiFailure catch (failure) {
      _showMessage('公告状态同步失败：${failure.message}');
    }
  }

  Future<void> _openAnnouncement(
    Announcement announcement,
    bool authenticated,
  ) async {
    if (_isMutating) {
      return;
    }
    setState(() => _isMutating = true);
    try {
      await _recordSeen(announcement, authenticated);
      if (!mounted) {
        return;
      }
      final AnnouncementReceiptInputActionEnum? action =
          await showAnnouncementDialog(
            context: context,
            announcement: announcement,
          );
      if (authenticated) {
        await _recordAction(
          announcement,
          action ?? AnnouncementReceiptInputActionEnum.dismiss,
        );
        await _load();
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _acknowledge(Announcement announcement) async {
    if (_isMutating) {
      return;
    }
    setState(() => _isMutating = true);
    try {
      await _repository.record(
        announcement: announcement,
        action: AnnouncementReceiptInputActionEnum.acknowledge,
      );
      if (mounted) {
        _showMessage('已确认公告');
        await _load();
      }
    } on ApiFailure catch (failure) {
      _showMessage(failure.message);
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  void _showMessage(String message) {
    if (mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(message)));
    }
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final AsyncValue<SessionState> session = ref.watch(sessionStateProvider);
    final SessionState? state = session.value;
    if (state != null && state.generation != _sessionGeneration) {
      _sessionGeneration = state.generation;
      ++_requestGeneration;
      _announcements = <Announcement>[];
      _failure = null;
      _isLoading = true;
      final int expectedGeneration = state.generation;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (mounted &&
            ref.read(sessionStateProvider).value?.generation ==
                expectedGeneration) {
          unawaited(_load());
        }
      });
    }
    final Widget content = _content(state);
    if (widget.embedded) {
      return content;
    }
    return Scaffold(
      appBar: AppBar(title: const Text('社区公告')),
      body: SafeArea(top: false, child: content),
    );
  }

  Widget _content(SessionState? session) {
    if (session == null || session.phase == SessionPhase.restoring) {
      return const AppLoadingState(title: '正在确认公告受众');
    }
    if (_isLoading && _announcements.isEmpty) {
      return const AppLoadingState(title: '正在加载社区公告');
    }
    final ApiFailure? failure = _failure;
    if (failure != null && _announcements.isEmpty) {
      if (failure.kind == ApiFailureKind.forbidden) {
        return const AppPermissionState(description: '当前账号不在这些公告的受众范围内。');
      }
      return AppErrorState(description: failure.message, onRetry: _load);
    }
    if (_announcements.isEmpty) {
      return const AppEmptyState(
        title: '当前没有生效中的公告',
        description: '新公告发布后会显示在这里。',
      );
    }
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView.separated(
        key: const PageStorageKey<String>('announcements-scroll'),
        padding: const EdgeInsets.all(16),
        itemCount: _announcements.length + 1,
        separatorBuilder: (_, _) => const SizedBox(height: 12),
        itemBuilder: (BuildContext context, int index) {
          if (index == 0) {
            return _AnnouncementHeader(
              isAuthenticated: session.isAuthenticated,
              onLogin: () => context.push(
                publicInteractionLoginLocation(GoRouterState.of(context).uri),
              ),
            );
          }
          final Announcement announcement = _announcements[index - 1];
          return _AnnouncementCard(
            announcement: announcement,
            isAuthenticated: session.isAuthenticated,
            isBusy: _isMutating,
            onOpen: () =>
                _openAnnouncement(announcement, session.isAuthenticated),
            onAcknowledge: () => _acknowledge(announcement),
          );
        },
      ),
    );
  }
}

class _AnnouncementHeader extends StatelessWidget {
  const _AnnouncementHeader({
    required this.isAuthenticated,
    required this.onLogin,
  });

  final bool isAuthenticated;
  final VoidCallback onLogin;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Text('社区公告', style: Theme.of(context).textTheme.headlineSmall),
        const SizedBox(height: 4),
        const Text('查看当前对你生效的平台公告；重大修订会按 revision 重新展示。'),
        if (!isAuthenticated) ...<Widget>[
          const SizedBox(height: 12),
          Card(
            child: ListTile(
              leading: const Icon(Icons.info_outline_rounded),
              title: const Text('当前按匿名受众显示'),
              subtitle: const Text('登录后，查看、稍后处理和确认状态由服务器跨设备保存。'),
              trailing: TextButton(onPressed: onLogin, child: const Text('登录')),
            ),
          ),
        ],
      ],
    );
  }
}

class _AnnouncementCard extends StatelessWidget {
  const _AnnouncementCard({
    required this.announcement,
    required this.isAuthenticated,
    required this.isBusy,
    required this.onOpen,
    required this.onAcknowledge,
  });

  final Announcement announcement;
  final bool isAuthenticated;
  final bool isBusy;
  final VoidCallback onOpen;
  final VoidCallback onAcknowledge;

  @override
  Widget build(BuildContext context) {
    final bool acknowledged = announcement.receipt?.acknowledgedAt != null;
    final _SeverityMeta severity = _severityMeta(announcement.severity);
    return Card(
      shape: announcement.severity == AnnouncementSeverityEnum.critical
          ? RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(12),
              side: BorderSide(color: Theme.of(context).colorScheme.error),
            )
          : null,
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        onTap: isBusy ? null : onOpen,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: <Widget>[
              Wrap(
                spacing: 8,
                runSpacing: 8,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: <Widget>[
                  Chip(
                    avatar: Icon(severity.icon, size: 18),
                    label: Text(severity.label),
                  ),
                  Chip(label: Text('版本 ${announcement.revision}')),
                  if (announcement.receipt?.firstSeenAt != null)
                    const Chip(label: Text('已查看')),
                  if (acknowledged) const Chip(label: Text('已确认')),
                ],
              ),
              const SizedBox(height: 10),
              Text(
                announcement.title,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              if (announcement.body case final String body) ...<Widget>[
                const SizedBox(height: 10),
                Text(body, maxLines: 5, overflow: TextOverflow.ellipsis),
              ],
              const SizedBox(height: 12),
              Text(
                '有效期：${_scheduleText(announcement)}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
              if (isAuthenticated &&
                  announcement.requiresAck &&
                  !acknowledged) ...<Widget>[
                const SizedBox(height: 12),
                Align(
                  alignment: Alignment.centerRight,
                  child: FilledButton.icon(
                    onPressed: isBusy ? null : onAcknowledge,
                    icon: const Icon(Icons.check_circle_outline_rounded),
                    label: const Text('我已知晓'),
                  ),
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

class _SeverityMeta {
  const _SeverityMeta(this.label, this.icon);

  final String label;
  final IconData icon;
}

_SeverityMeta _severityMeta(AnnouncementSeverityEnum severity) =>
    switch (severity) {
      AnnouncementSeverityEnum.info => const _SeverityMeta(
        '平台信息',
        Icons.info_outline_rounded,
      ),
      AnnouncementSeverityEnum.success => const _SeverityMeta(
        '平台进展',
        Icons.check_circle_outline_rounded,
      ),
      AnnouncementSeverityEnum.warning => const _SeverityMeta(
        '重要提醒',
        Icons.warning_amber_rounded,
      ),
      AnnouncementSeverityEnum.critical => const _SeverityMeta(
        '紧急公告',
        Icons.shield_outlined,
      ),
      AnnouncementSeverityEnum.unknownDefaultOpenApi => const _SeverityMeta(
        '公告',
        Icons.campaign_outlined,
      ),
    };

String _scheduleText(Announcement announcement) {
  return announcementScheduleText(announcement);
}
