import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../auth/domain/session_state.dart';
import '../data/forum_repository.dart';
import 'forum_widgets.dart';

/// Selects the owner API used to read a bounded forum revision page.
enum ForumRevisionTarget { thread, comment }

/// Opens the permission-gated revision reader without exposing raw media data.
Future<void> showForumRevisionHistorySheet({
  required BuildContext context,
  required ForumRepository repository,
  required ForumRevisionTarget target,
  required String targetId,
}) {
  return showModalBottomSheet<void>(
    context: context,
    isScrollControlled: true,
    useSafeArea: true,
    builder: (BuildContext context) => FractionallySizedBox(
      heightFactor: 0.9,
      child: ForumRevisionHistorySheet(
        repository: repository,
        target: target,
        targetId: targetId,
      ),
    ),
  );
}

/// Renders cursor-paginated edit snapshots returned by the Forum owner API.
class ForumRevisionHistorySheet extends ConsumerStatefulWidget {
  const ForumRevisionHistorySheet({
    required this.repository,
    required this.target,
    required this.targetId,
    super.key,
  });

  final ForumRepository repository;
  final ForumRevisionTarget target;
  final String targetId;

  @override
  ConsumerState<ForumRevisionHistorySheet> createState() =>
      _ForumRevisionHistorySheetState();
}

class _ForumRevisionHistorySheetState
    extends ConsumerState<ForumRevisionHistorySheet> {
  final List<PostRevision> _items = <PostRevision>[];
  (int, SessionPhase, String?)? _sessionIdentity;
  SessionState? _session;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  ApiFailure? _error;
  ApiFailure? _loadMoreError;
  bool _hasRequestedDeliveryRefresh = false;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    ref.listenManual<AsyncValue<SessionState>>(
      sessionStateProvider,
      _handleSessionState,
      fireImmediately: true,
    );
  }

  @override
  void didUpdateWidget(ForumRevisionHistorySheet oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.target != widget.target ||
        oldWidget.targetId != widget.targetId ||
        oldWidget.repository != widget.repository) {
      _generation += 1;
      WidgetsBinding.instance.addPostFrameCallback((Duration _) {
        if (mounted) {
          _resetForCurrentTarget();
        }
      });
    }
  }

  void _handleSessionState(
    AsyncValue<SessionState>? _,
    AsyncValue<SessionState> next,
  ) {
    final SessionState? state = next.value;
    if (state == null) {
      return;
    }
    final (int, SessionPhase, String?) identity = (
      state.generation,
      state.phase,
      state.account?.id,
    );
    if (_sessionIdentity == identity) {
      return;
    }
    _sessionIdentity = identity;
    _session = state;
    _generation += 1;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted && _sessionIdentity == identity) {
        _resetForCurrentTarget();
      }
    });
  }

  void _resetForCurrentTarget({bool resetDeliveryRefresh = true}) {
    _generation += 1;
    final bool canRequest = _session?.isAuthenticated ?? false;
    setState(() {
      _items.clear();
      _nextCursor = null;
      _hasMore = false;
      _isLoading = canRequest;
      _isLoadingMore = false;
      _error = null;
      _loadMoreError = null;
      if (resetDeliveryRefresh) {
        _hasRequestedDeliveryRefresh = false;
      }
    });
    if (canRequest) {
      unawaited(_loadPage());
    }
  }

  Future<void> _reload() async {
    _resetForCurrentTarget();
  }

  void _refreshDeliveryOnce() {
    if (_hasRequestedDeliveryRefresh) {
      return;
    }
    _hasRequestedDeliveryRefresh = true;
    _resetForCurrentTarget(resetDeliveryRefresh: false);
  }

  Future<void> _loadPage({bool loadMore = false}) async {
    if (!(_session?.isAuthenticated ?? false)) {
      return;
    }
    if (loadMore) {
      if (_isLoadingMore || !_hasMore || _nextCursor == null) {
        return;
      }
      setState(() {
        _isLoadingMore = true;
        _loadMoreError = null;
      });
    }
    final int generation = _generation;
    final String targetId = widget.targetId;
    final ForumRevisionTarget target = widget.target;
    final String? cursor = loadMore ? _nextCursor : null;
    try {
      final ForumPageSlice<PostRevision> page = await switch (target) {
        ForumRevisionTarget.thread => widget.repository.threadRevisions(
          targetId,
          cursor: cursor,
        ),
        ForumRevisionTarget.comment => widget.repository.commentRevisions(
          targetId,
          cursor: cursor,
        ),
      };
      if (!mounted ||
          generation != _generation ||
          target != widget.target ||
          targetId != widget.targetId) {
        return;
      }
      setState(() {
        if (loadMore) {
          final Set<String> known = _items
              .map((PostRevision revision) => revision.id)
              .toSet();
          _items.addAll(
            page.items.where((PostRevision revision) => known.add(revision.id)),
          );
        } else {
          _items
            ..clear()
            ..addAll(page.items);
        }
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore && page.nextCursor != null;
        _error = null;
        _loadMoreError = null;
      });
    } on ApiFailure catch (failure) {
      if (!mounted || generation != _generation) {
        return;
      }
      setState(() {
        if (loadMore) {
          _loadMoreError = failure;
        } else {
          _error = failure;
        }
      });
    } finally {
      if (mounted && generation == _generation) {
        setState(() {
          _isLoading = false;
          _isLoadingMore = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        automaticallyImplyLeading: false,
        title: Text(
          widget.target == ForumRevisionTarget.thread ? '主题修订历史' : '回复修订历史',
        ),
        actions: <Widget>[
          IconButton(
            tooltip: '关闭修订历史',
            onPressed: () => Navigator.of(context).pop(),
            icon: const Icon(Icons.close_rounded),
          ),
        ],
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    final SessionState? session = _session;
    if (session == null || session.phase == SessionPhase.restoring) {
      return const AppLoadingState(title: '正在确认修订历史权限');
    }
    if (!session.isAuthenticated) {
      return const AppPermissionState(
        title: '登录后查看修订历史',
        description: '历史只向内容作者本人，或满足能力与层级约束的工作人员开放。',
      );
    }
    if (_isLoading) {
      return const AppLoadingState(title: '加载修订历史');
    }
    if (_error case final ApiFailure failure) {
      if (failure.kind == ApiFailureKind.forbidden ||
          failure.kind == ApiFailureKind.unauthorized) {
        return AppPermissionState(
          title: '无权查看修订历史',
          description: failure.message,
        );
      }
      return AppErrorState(
        title: '修订历史加载失败',
        description: failure.message,
        onRetry: _reload,
      );
    }
    if (_items.isEmpty) {
      return const AppEmptyState(
        title: '暂无修订历史',
        description: '内容尚未编辑；当前正文不是一条历史快照。',
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 32),
      itemCount: _items.length + 1,
      itemBuilder: (BuildContext context, int index) {
        if (index < _items.length) {
          final PostRevision revision = _items[index];
          return Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: _RevisionCard(
              revision: revision,
              initiallyExpanded: index == 0,
              onRefreshDelivery: _refreshDeliveryOnce,
            ),
          );
        }
        if (_loadMoreError case final ApiFailure failure) {
          return Semantics(
            liveRegion: true,
            child: Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  children: <Widget>[
                    Text('更多历史加载失败：${failure.message}'),
                    const SizedBox(height: 8),
                    OutlinedButton.icon(
                      onPressed: () => _loadPage(loadMore: true),
                      icon: const Icon(Icons.refresh_rounded),
                      label: const Text('重试加载更多'),
                    ),
                  ],
                ),
              ),
            ),
          );
        }
        if (!_hasMore) {
          return const SizedBox.shrink();
        }
        return OutlinedButton.icon(
          onPressed: _isLoadingMore ? null : () => _loadPage(loadMore: true),
          icon: _isLoadingMore
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.expand_more_rounded),
          label: Text(_isLoadingMore ? '加载中' : '加载更多历史'),
        );
      },
    );
  }
}

class _RevisionCard extends StatelessWidget {
  const _RevisionCard({
    required this.revision,
    required this.initiallyExpanded,
    required this.onRefreshDelivery,
  });

  final PostRevision revision;
  final bool initiallyExpanded;
  final VoidCallback onRefreshDelivery;

  @override
  Widget build(BuildContext context) {
    return Card(
      clipBehavior: Clip.antiAlias,
      child: ExpansionTile(
        initiallyExpanded: initiallyExpanded,
        title: Text('编辑前版本 v${revision.oldContentVersion}'),
        subtitle: Text(
          '修订 #${revision.seq} · ${formatForumTime(revision.createdAt)}',
        ),
        childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        expandedCrossAxisAlignment: CrossAxisAlignment.start,
        children: <Widget>[
          if (revision.oldTitle case final String title) ...<Widget>[
            Text(
              title,
              style: Theme.of(
                context,
              ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 12),
          ],
          ForumBody(
            source: revision.oldBody,
            format: revision.oldContentFormat,
            attachments: revision.attachments,
            onRefreshDelivery: onRefreshDelivery,
          ),
        ],
      ),
    );
  }
}
