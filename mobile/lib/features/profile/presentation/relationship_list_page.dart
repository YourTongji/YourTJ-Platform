import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../../core/widgets/platform_avatar.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';

enum RelationshipListKind { followers, following }

class RelationshipListPage extends ConsumerStatefulWidget {
  const RelationshipListPage({
    required this.handle,
    required this.kind,
    super.key,
  });

  final String handle;
  final RelationshipListKind kind;

  @override
  ConsumerState<RelationshipListPage> createState() =>
      _RelationshipListPageState();
}

class _RelationshipListPageState extends ConsumerState<RelationshipListPage> {
  final List<UserSummary> _items = <UserSummary>[];
  ApiFailure? _failure;
  String? _nextCursor;
  bool _hasMore = false;
  bool _isLoading = true;
  bool _isLoadingMore = false;
  String? _removingHandle;

  @override
  void initState() {
    super.initState();
    _load(reset: true);
  }

  Future<void> _load({required bool reset}) async {
    if (reset) {
      setState(() {
        _isLoading = true;
        _failure = null;
      });
    } else {
      if (_isLoadingMore || !_hasMore) {
        return;
      }
      setState(() => _isLoadingMore = true);
    }
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      final UserSummaryPage page = widget.kind == RelationshipListKind.followers
          ? await repository.getFollowers(
              widget.handle,
              cursor: reset ? null : _nextCursor,
            )
          : await repository.getFollowing(
              widget.handle,
              cursor: reset ? null : _nextCursor,
            );
      if (!mounted) {
        return;
      }
      setState(() {
        if (reset) {
          _items.clear();
        }
        _items.addAll(page.items);
        _nextCursor = page.nextCursor;
        _hasMore = page.hasMore;
        _failure = null;
      });
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _isLoadingMore = false;
        });
      }
    }
  }

  Future<void> _removeFollower(UserSummary user) async {
    final bool? confirmed = await showDialog<bool>(
      context: context,
      builder: (BuildContext dialogContext) => AlertDialog(
        title: Text('移除 @${user.handle} 的关注？'),
        content: const Text('这只会删除对方对你的关注，不会屏蔽对方，也不会改变你是否关注对方。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('移除关注者'),
          ),
        ],
      ),
    );
    if (confirmed != true || !mounted) {
      return;
    }
    setState(() {
      _removingHandle = user.handle;
      _failure = null;
    });
    try {
      await ref.read(accountRepositoryProvider).removeFollower(user.handle);
      if (mounted) {
        setState(
          () => _items.removeWhere((UserSummary item) => item.id == user.id),
        );
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _removingHandle = null);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final String title = widget.kind == RelationshipListKind.followers
        ? '@${widget.handle} 的关注者'
        : '@${widget.handle} 正在关注';
    final Widget child;
    if (_isLoading) {
      child = AppLoadingState(title: '正在读取$title');
    } else if (_items.isEmpty && _failure != null) {
      child = AccountFailureView(
        failure: _failure!,
        onRetry: () => _load(reset: true),
      );
    } else if (_items.isEmpty) {
      child = AppEmptyState(
        title: widget.kind == RelationshipListKind.followers
            ? '还没有可见的关注者'
            : '还没有可见的关注账号',
        description: '列表内容受资料和关系列表隐私选择控制。',
      );
    } else {
      child = _buildList();
    }
    return AccountPageLayout(title: title, child: child);
  }

  Widget _buildList() {
    final String? currentHandle = ref
        .watch(sessionStateProvider)
        .value
        ?.account
        ?.handle;
    final bool canRemove =
        widget.kind == RelationshipListKind.followers &&
        currentHandle?.toLowerCase() == widget.handle.toLowerCase();
    return RefreshIndicator(
      onRefresh: () => _load(reset: true),
      child: ListView.builder(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(16),
        itemCount: _items.length + (_hasMore || _failure != null ? 1 : 0),
        itemBuilder: (BuildContext context, int index) {
          if (index == _items.length) {
            if (_failure != null) {
              return AppErrorState(
                title: '后续列表加载失败',
                description: _failure!.message,
                onRetry: () => _load(reset: false),
              );
            }
            return Padding(
              padding: const EdgeInsets.all(16),
              child: OutlinedButton(
                onPressed: _isLoadingMore ? null : () => _load(reset: false),
                child: _isLoadingMore
                    ? const CircularProgressIndicator()
                    : const Text('加载更多'),
              ),
            );
          }
          final UserSummary user = _items[index];
          final bool isRemoving = _removingHandle == user.handle;
          return Card(
            child: ListTile(
              leading: PlatformAvatar(
                compatibilityUrl: user.avatarUrl,
                fallbackText: user.handle,
                semanticLabel: '${user.handle} 的头像',
                onRefresh: () => _load(reset: true),
              ),
              title: Text(user.displayName ?? user.handle),
              subtitle: Text('@${user.handle} · ${user.role.value}'),
              onTap: () => context.push(AppRoutes.profile(user.handle)),
              trailing: canRemove
                  ? IconButton(
                      tooltip: '移除 @${user.handle} 关注者',
                      onPressed: isRemoving
                          ? null
                          : () => _removeFollower(user),
                      icon: isRemoving
                          ? const SizedBox.square(
                              dimension: 18,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.person_remove_outlined),
                    )
                  : const Icon(Icons.chevron_right_rounded),
            ),
          );
        },
      ),
    );
  }
}
