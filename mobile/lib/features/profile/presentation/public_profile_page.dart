import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/navigation/external_link.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../../core/widgets/platform_avatar.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';
import '../../auth/domain/session_state.dart';
import '../data/profile_activity_repository.dart';
import '../domain/profile_activity_controller.dart';
import 'profile_activity_section.dart';

class PublicProfilePage extends ConsumerStatefulWidget {
  const PublicProfilePage({required this.handle, super.key});

  final String handle;

  @override
  ConsumerState<PublicProfilePage> createState() => _PublicProfilePageState();
}

class _PublicProfilePageState extends ConsumerState<PublicProfilePage> {
  UserProfile? _profile;
  UserRelationship? _relationship;
  ApiFailure? _failure;
  ApiFailure? _relationshipFailure;
  bool _isLoading = true;
  bool _isMutating = false;
  late final ProfileActivityController _activityController;
  late (int, SessionPhase, String?) _viewerIdentity;
  int _profileRequestRevision = 0;

  @override
  void initState() {
    super.initState();
    final SessionState session = ref.read(sessionManagerProvider).state;
    _viewerIdentity = _identity(session);
    _activityController = ProfileActivityController(
      ref.read(profileActivityRepositoryProvider),
    );
    _activityController.configure(
      handle: widget.handle,
      viewerKey: _viewerKey(_viewerIdentity),
      canViewActivity: false,
    );
    ref.listenManual<AsyncValue<SessionState>>(
      sessionStateProvider,
      _handleSessionChange,
    );
    _load();
  }

  @override
  void didUpdateWidget(PublicProfilePage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.handle != widget.handle) {
      _profileRequestRevision += 1;
      _activityController.configure(
        handle: widget.handle,
        viewerKey: _viewerKey(_viewerIdentity),
        canViewActivity: false,
      );
      _load();
    }
  }

  @override
  void dispose() {
    _profileRequestRevision += 1;
    _activityController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final String requestedHandle = widget.handle;
    final (int, SessionPhase, String?) viewerIdentity = _viewerIdentity;
    final int requestRevision = ++_profileRequestRevision;
    _activityController.configure(
      handle: requestedHandle,
      viewerKey: _viewerKey(viewerIdentity),
      canViewActivity: false,
    );
    setState(() {
      _isLoading = true;
      _failure = null;
      _relationshipFailure = null;
    });
    final AccountRepository repository = ref.read(accountRepositoryProvider);
    try {
      final UserProfile profile = await repository.getUserProfile(
        requestedHandle,
      );
      UserRelationship? relationship;
      ApiFailure? relationshipFailure;
      if (viewerIdentity.$2 == SessionPhase.authenticated) {
        try {
          relationship = await repository.getRelationship(profile.handle);
        } on ApiFailure catch (failure) {
          relationshipFailure = failure;
        }
      }
      if (_isCurrentProfileRequest(
        requestRevision,
        requestedHandle,
        viewerIdentity,
      )) {
        setState(() {
          _profile = profile;
          _relationship = relationship;
          _relationshipFailure = relationshipFailure;
        });
        _activityController.configure(
          handle: profile.handle,
          viewerKey: _viewerKey(viewerIdentity),
          canViewActivity: profile.canViewActivity,
        );
        if (profile.canViewActivity) {
          unawaited(_activityController.loadSelected());
        }
      }
    } on ApiFailure catch (failure) {
      if (_isCurrentProfileRequest(
        requestRevision,
        requestedHandle,
        viewerIdentity,
      )) {
        setState(() => _failure = failure);
      }
    } finally {
      if (_isCurrentProfileRequest(
        requestRevision,
        requestedHandle,
        viewerIdentity,
      )) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _refreshRelationship() async {
    final UserProfile? profile = _profile;
    if (profile == null) {
      return;
    }
    final (int, SessionPhase, String?) viewerIdentity = _viewerIdentity;
    try {
      final UserRelationship relationship = await ref
          .read(accountRepositoryProvider)
          .getRelationship(profile.handle);
      if (mounted &&
          viewerIdentity == _viewerIdentity &&
          _profile?.id == profile.id) {
        setState(() {
          _relationship = relationship;
          _relationshipFailure = null;
        });
      }
    } on ApiFailure catch (failure) {
      if (mounted &&
          viewerIdentity == _viewerIdentity &&
          _profile?.id == profile.id) {
        setState(() => _relationshipFailure = failure);
      }
    }
  }

  Future<void> _toggleFollow() async {
    final UserProfile? profile = _profile;
    final UserRelationship? relationship = _relationship;
    if (profile == null || relationship == null || _isMutating) {
      return;
    }
    setState(() {
      _isMutating = true;
      _relationshipFailure = null;
    });
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      if (relationship.following) {
        await repository.unfollow(profile.handle);
      } else {
        await repository.follow(profile.handle);
      }
      await _refreshRelationship();
      if (mounted) {
        await _reloadProfileFacts();
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _relationshipFailure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _toggleMute() async {
    final UserProfile? profile = _profile;
    final UserRelationship? relationship = _relationship;
    if (profile == null || relationship == null || _isMutating) {
      return;
    }
    setState(() {
      _isMutating = true;
      _relationshipFailure = null;
    });
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      if (relationship.muted) {
        await repository.unmute(profile.handle);
      } else {
        await repository.mute(profile.handle);
      }
      await _refreshRelationship();
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _relationshipFailure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _toggleBlock() async {
    final UserProfile? profile = _profile;
    final UserRelationship? relationship = _relationship;
    if (profile == null || relationship == null || _isMutating) {
      return;
    }
    if (!relationship.blockedByMe) {
      final bool? confirmed = await showDialog<bool>(
        context: context,
        builder: (BuildContext dialogContext) => AlertDialog(
          title: Text('屏蔽 @${profile.handle}？'),
          content: const Text('屏蔽会建立双向安全边界，移除双方关注，并阻止新互动。以后解除屏蔽也不会自动恢复关注。'),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.pop(dialogContext, false),
              child: const Text('取消'),
            ),
            FilledButton(
              onPressed: () => Navigator.pop(dialogContext, true),
              child: const Text('确认屏蔽'),
            ),
          ],
        ),
      );
      if (confirmed != true || !mounted) {
        return;
      }
    }
    setState(() {
      _isMutating = true;
      _relationshipFailure = null;
    });
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      if (relationship.blockedByMe) {
        await repository.unblock(profile.handle);
      } else {
        await repository.block(profile.handle);
      }
      await _refreshRelationship();
      if (mounted) {
        await _reloadProfileFacts();
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _relationshipFailure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isMutating = false);
      }
    }
  }

  Future<void> _reloadProfileFacts() async {
    final UserProfile? profile = _profile;
    if (profile == null) {
      return;
    }
    final (int, SessionPhase, String?) viewerIdentity = _viewerIdentity;
    try {
      final UserProfile current = await ref
          .read(accountRepositoryProvider)
          .getUserProfile(profile.handle);
      if (mounted &&
          viewerIdentity == _viewerIdentity &&
          _profile?.id == profile.id) {
        setState(() => _profile = current);
      }
    } on ApiFailure catch (failure) {
      if (mounted &&
          viewerIdentity == _viewerIdentity &&
          _profile?.id == profile.id) {
        setState(() => _relationshipFailure = failure);
      }
    }
  }

  Future<void> _openWebsite(String website) async {
    final Uri? uri = Uri.tryParse(website);
    if (uri == null || !isAllowedExternalHttps(uri)) {
      return;
    }
    await confirmAndOpenExternalHttps(context, uri);
  }

  Future<void> _loginAndLoadRelationship() async {
    await context.push(
      publicInteractionLoginLocation(GoRouterState.of(context).uri),
    );
    if (!mounted || !ref.read(sessionManagerProvider).state.isAuthenticated) {
      return;
    }
    await _refreshRelationship();
  }

  void _handleSessionChange(
    AsyncValue<SessionState>? previous,
    AsyncValue<SessionState> next,
  ) {
    final SessionState? session = next.value;
    if (session == null) {
      return;
    }
    final (int, SessionPhase, String?) identity = _identity(session);
    if (identity == _viewerIdentity) {
      return;
    }
    _viewerIdentity = identity;
    _profileRequestRevision += 1;
    _activityController.configure(
      handle: widget.handle,
      viewerKey: _viewerKey(identity),
      canViewActivity: false,
    );
    unawaited(_load());
  }

  bool _isCurrentProfileRequest(
    int requestRevision,
    String requestedHandle,
    (int, SessionPhase, String?) viewerIdentity,
  ) {
    return mounted &&
        requestRevision == _profileRequestRevision &&
        requestedHandle == widget.handle &&
        viewerIdentity == _viewerIdentity;
  }

  (int, SessionPhase, String?) _identity(SessionState session) =>
      (session.generation, session.phase, session.account?.id);

  String _viewerKey((int, SessionPhase, String?) identity) =>
      '${identity.$1}:${identity.$2.name}:${identity.$3 ?? 'anonymous'}';

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取个人主页',
        description: '服务器会按当前访问者与资料隐私策略返回内容。',
      );
    } else if (_profile == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildProfile(context, _profile!);
    }
    return AccountPageLayout(title: '个人主页', maxWidth: 880, child: child);
  }

  Widget _buildProfile(BuildContext context, UserProfile profile) {
    final UserRelationship? relationship = _relationship;
    final bool isAuthenticated =
        ref.watch(sessionStateProvider).value?.isAuthenticated ??
        ref.read(sessionManagerProvider).state.isAuthenticated;
    return RefreshIndicator(
      onRefresh: _load,
      child: ListView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.all(24),
        children: <Widget>[
          Card(
            clipBehavior: Clip.antiAlias,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                if (profile.bannerUrl != null)
                  PlatformImage(
                    url: profile.bannerUrl!,
                    height: 160,
                    semanticLabel: '${profile.handle} 的个人主页封面',
                    onRefresh: _reloadProfileFacts,
                  ),
                Padding(
                  padding: const EdgeInsets.all(20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      Row(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: <Widget>[
                          _ProfileAvatar(
                            profile: profile,
                            onRefresh: _reloadProfileFacts,
                          ),
                          const SizedBox(width: 16),
                          Expanded(
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: <Widget>[
                                Semantics(
                                  header: true,
                                  child: Text(
                                    profile.displayName ?? profile.handle,
                                    style: Theme.of(
                                      context,
                                    ).textTheme.headlineSmall,
                                  ),
                                ),
                                Text('@${profile.handle}'),
                                const SizedBox(height: 4),
                                Text(
                                  '${profile.school} · 信任等级 ${profile.trustLevel}',
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                      if (profile.bio != null) ...<Widget>[
                        const SizedBox(height: 16),
                        Text(profile.bio!),
                      ],
                      if (profile.website != null) ...<Widget>[
                        const SizedBox(height: 8),
                        Align(
                          alignment: Alignment.centerLeft,
                          child: TextButton.icon(
                            onPressed: () => _openWebsite(profile.website!),
                            icon: const Icon(Icons.open_in_new_rounded),
                            label: Text(profile.website!),
                          ),
                        ),
                      ],
                      if (profile.verifications.isNotEmpty ||
                          profile.badges.isNotEmpty) ...<Widget>[
                        const SizedBox(height: 12),
                        Wrap(
                          spacing: 8,
                          runSpacing: 8,
                          children: <Widget>[
                            ...profile.verifications.map(
                              (PublicVerification verification) => Chip(
                                avatar: const Icon(
                                  Icons.verified_rounded,
                                  size: 18,
                                ),
                                label: Text(verification.label),
                              ),
                            ),
                            ...profile.badges.map(
                              (UserBadge badge) => Chip(
                                avatar: const Icon(
                                  Icons.workspace_premium_outlined,
                                  size: 18,
                                ),
                                label: Text(badge.name),
                              ),
                            ),
                          ],
                        ),
                      ],
                      const SizedBox(height: 16),
                      Wrap(
                        spacing: 12,
                        runSpacing: 12,
                        children: <Widget>[
                          _StatButton(
                            label: '关注者',
                            value: profile.followerCount,
                            onPressed: () => context.push(
                              AppRoutes.profileFollowers(profile.handle),
                            ),
                          ),
                          _StatButton(
                            label: '正在关注',
                            value: profile.followingCount,
                            onPressed: () => context.push(
                              AppRoutes.profileFollowing(profile.handle),
                            ),
                          ),
                          _StatButton(label: '主题', value: profile.threadCount),
                          _StatButton(label: '评论', value: profile.commentCount),
                          _StatButton(
                            label: '获赞',
                            value: profile.votesReceived,
                          ),
                        ],
                      ),
                      const SizedBox(height: 16),
                      if (!isAuthenticated)
                        FilledButton.icon(
                          onPressed: _loginAndLoadRelationship,
                          icon: const Icon(Icons.login_rounded),
                          label: const Text('登录后关注或管理安全边界'),
                        )
                      else if (relationship != null && !relationship.isSelf)
                        _RelationshipActions(
                          relationship: relationship,
                          isMutating: _isMutating,
                          onFollow: _toggleFollow,
                          onMute: _toggleMute,
                          onBlock: _toggleBlock,
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 18),
          ProfileActivitySection(controller: _activityController),
          if (_relationshipFailure != null) ...<Widget>[
            const SizedBox(height: 16),
            AppErrorState(
              title: '社交状态操作失败',
              description: _relationshipFailure!.message,
              onRetry: _refreshRelationship,
            ),
          ],
        ],
      ),
    );
  }
}

class _ProfileAvatar extends StatelessWidget {
  const _ProfileAvatar({required this.profile, required this.onRefresh});

  final UserProfile profile;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    return PlatformAvatar(
      radius: 36,
      compatibilityUrl: profile.avatarUrl,
      fallbackText: profile.handle,
      semanticLabel: '${profile.handle} 的头像',
      onRefresh: onRefresh,
    );
  }
}

class _StatButton extends StatelessWidget {
  const _StatButton({required this.label, required this.value, this.onPressed});

  final String label;
  final int value;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return OutlinedButton(onPressed: onPressed, child: Text('$value $label'));
  }
}

class _RelationshipActions extends StatelessWidget {
  const _RelationshipActions({
    required this.relationship,
    required this.isMutating,
    required this.onFollow,
    required this.onMute,
    required this.onBlock,
  });

  final UserRelationship relationship;
  final bool isMutating;
  final VoidCallback onFollow;
  final VoidCallback onMute;
  final VoidCallback onBlock;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 12,
      runSpacing: 12,
      children: <Widget>[
        FilledButton.icon(
          onPressed:
              isMutating || (!relationship.following && !relationship.canFollow)
              ? null
              : onFollow,
          icon: Icon(
            relationship.following
                ? Icons.person_remove_outlined
                : Icons.person_add_alt,
          ),
          label: Text(relationship.following ? '取消关注' : '关注'),
        ),
        OutlinedButton.icon(
          onPressed: isMutating ? null : onMute,
          icon: Icon(
            relationship.muted
                ? Icons.volume_up_outlined
                : Icons.volume_off_outlined,
          ),
          label: Text(relationship.muted ? '取消隐藏' : '隐藏内容'),
        ),
        OutlinedButton.icon(
          onPressed: isMutating ? null : onBlock,
          icon: Icon(
            relationship.blockedByMe
                ? Icons.lock_open_rounded
                : Icons.block_rounded,
          ),
          label: Text(relationship.blockedByMe ? '解除屏蔽' : '屏蔽'),
        ),
      ],
    );
  }
}
