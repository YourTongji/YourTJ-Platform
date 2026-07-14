import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';

class PrivacySettingsPage extends ConsumerStatefulWidget {
  const PrivacySettingsPage({super.key});

  @override
  ConsumerState<PrivacySettingsPage> createState() =>
      _PrivacySettingsPageState();
}

class _PrivacySettingsPageState extends ConsumerState<PrivacySettingsPage> {
  ProfilePrivacy? _privacy;
  ApiFailure? _failure;
  bool _isLoading = true;
  bool _isSaving = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final ProfilePrivacy privacy = await ref
          .read(accountRepositoryProvider)
          .getPrivacy();
      if (mounted) {
        setState(() => _privacy = _normalizePrivacy(privacy));
      }
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _save() async {
    final ProfilePrivacy? privacy = _privacy;
    if (privacy == null || _isSaving) {
      return;
    }
    setState(() {
      _isSaving = true;
      _failure = null;
    });
    try {
      final ProfilePrivacy saved = await ref
          .read(accountRepositoryProvider)
          .updatePrivacy(
            ProfilePrivacyUpdateInput(
              profileVisibility: privacy.profileVisibility,
              activityVisibility: privacy.activityVisibility,
              followersVisibility: privacy.followersVisibility,
              followingVisibility: privacy.followingVisibility,
              discoverable: privacy.discoverable,
              dmPolicy: privacy.dmPolicy,
              mentionPolicy: privacy.mentionPolicy,
            ),
          );
      if (!mounted) {
        return;
      }
      setState(() => _privacy = _normalizePrivacy(saved));
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('隐私设置已与服务器同步')));
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isSaving = false);
      }
    }
  }

  void _update(ProfilePrivacy Function(ProfilePrivacy current) change) {
    final ProfilePrivacy? current = _privacy;
    if (current != null) {
      setState(() => _privacy = change(current));
    }
  }

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取隐私设置',
        description: '正在同步资料、活动、关系列表与联系偏好。',
      );
    } else if (_privacy == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildSettings(context, _privacy!);
    }
    return AccountPageLayout(title: '隐私与社交权限', child: child);
  }

  Widget _buildSettings(BuildContext context, ProfilePrivacy privacy) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: <Widget>[
        const Text('这些选择只改变对方能看到或能否发起交互的范围；实际访问每次仍由服务器校验。'),
        const SizedBox(height: 20),
        _EnumSetting<ProfileVisibility>(
          title: '个人资料可见性',
          description: '控制显示名、学校、简介和网站等资料。',
          value: privacy.profileVisibility,
          items: _profileVisibilityItems,
          onChanged: (ProfileVisibility value) => _update(
            (ProfilePrivacy current) =>
                _copyPrivacy(current, profileVisibility: value),
          ),
        ),
        _EnumSetting<ActivityVisibility>(
          title: '主页活动列表',
          description: '不改变原帖子或评论自身的可见性。',
          value: privacy.activityVisibility,
          items: _activityVisibilityItems,
          onChanged: (ActivityVisibility value) => _update(
            (ProfilePrivacy current) =>
                _copyPrivacy(current, activityVisibility: value),
          ),
        ),
        _EnumSetting<RelationshipListVisibility>(
          title: '关注者列表',
          description: '选择谁能查看关注你的账号。',
          value: privacy.followersVisibility,
          items: _relationshipVisibilityItems,
          onChanged: (RelationshipListVisibility value) => _update(
            (ProfilePrivacy current) =>
                _copyPrivacy(current, followersVisibility: value),
          ),
        ),
        _EnumSetting<RelationshipListVisibility>(
          title: '正在关注列表',
          description: '选择谁能查看你正在关注的账号。',
          value: privacy.followingVisibility,
          items: _relationshipVisibilityItems,
          onChanged: (RelationshipListVisibility value) => _update(
            (ProfilePrivacy current) =>
                _copyPrivacy(current, followingVisibility: value),
          ),
        ),
        _EnumSetting<DmPolicy>(
          title: '新私信',
          description: '已有会话不因这项设置自动删除。',
          value: privacy.dmPolicy,
          items: _dmPolicyItems,
          onChanged: (DmPolicy value) => _update(
            (ProfilePrivacy current) => _copyPrivacy(current, dmPolicy: value),
          ),
        ),
        _EnumSetting<MentionPolicy>(
          title: '语义提及通知',
          description: '不允许的 @handle 会保留为普通文字，但不生成提及通知。',
          value: privacy.mentionPolicy,
          items: _mentionPolicyItems,
          onChanged: (MentionPolicy value) => _update(
            (ProfilePrivacy current) =>
                _copyPrivacy(current, mentionPolicy: value),
          ),
        ),
        Card(
          child: SwitchListTile.adaptive(
            title: const Text('允许被发现'),
            subtitle: const Text('允许出现在第三方关系列表与未来账号搜索中；精确 handle 直达仍受资料权限控制。'),
            value: privacy.discoverable,
            onChanged: _isSaving
                ? null
                : (bool value) => _update(
                    (ProfilePrivacy current) =>
                        _copyPrivacy(current, discoverable: value),
                  ),
          ),
        ),
        if (_failure != null) ...<Widget>[
          const SizedBox(height: 16),
          Semantics(
            liveRegion: true,
            child: Text(
              _failure!.message,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
        ],
        const SizedBox(height: 24),
        FilledButton.icon(
          onPressed: _isSaving ? null : _save,
          icon: _isSaving
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.shield_outlined),
          label: Text(_isSaving ? '正在保存' : '保存隐私设置'),
        ),
      ],
    );
  }
}

class _EnumSetting<T> extends StatelessWidget {
  const _EnumSetting({
    required this.title,
    required this.description,
    required this.value,
    required this.items,
    required this.onChanged,
  });

  final String title;
  final String description;
  final T value;
  final List<DropdownMenuItem<T>> items;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Text(title, style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(description),
            const SizedBox(height: 12),
            DropdownButtonFormField<T>(
              initialValue: value,
              items: items,
              onChanged: (T? next) {
                if (next != null) {
                  onChanged(next);
                }
              },
            ),
          ],
        ),
      ),
    );
  }
}

ProfilePrivacy _normalizePrivacy(ProfilePrivacy privacy) {
  return _copyPrivacy(
    privacy,
    profileVisibility:
        privacy.profileVisibility == ProfileVisibility.unknownDefaultOpenApi
        ? ProfileVisibility.onlyMe
        : privacy.profileVisibility,
    activityVisibility:
        privacy.activityVisibility == ActivityVisibility.unknownDefaultOpenApi
        ? ActivityVisibility.onlyMe
        : privacy.activityVisibility,
    followersVisibility:
        privacy.followersVisibility ==
            RelationshipListVisibility.unknownDefaultOpenApi
        ? RelationshipListVisibility.onlyMe
        : privacy.followersVisibility,
    followingVisibility:
        privacy.followingVisibility ==
            RelationshipListVisibility.unknownDefaultOpenApi
        ? RelationshipListVisibility.onlyMe
        : privacy.followingVisibility,
    dmPolicy: privacy.dmPolicy == DmPolicy.unknownDefaultOpenApi
        ? DmPolicy.nobody
        : privacy.dmPolicy,
    mentionPolicy: privacy.mentionPolicy == MentionPolicy.unknownDefaultOpenApi
        ? MentionPolicy.nobody
        : privacy.mentionPolicy,
  );
}

ProfilePrivacy _copyPrivacy(
  ProfilePrivacy privacy, {
  ProfileVisibility? profileVisibility,
  ActivityVisibility? activityVisibility,
  RelationshipListVisibility? followersVisibility,
  RelationshipListVisibility? followingVisibility,
  bool? discoverable,
  DmPolicy? dmPolicy,
  MentionPolicy? mentionPolicy,
}) {
  return ProfilePrivacy(
    profileVisibility: profileVisibility ?? privacy.profileVisibility,
    activityVisibility: activityVisibility ?? privacy.activityVisibility,
    followersVisibility: followersVisibility ?? privacy.followersVisibility,
    followingVisibility: followingVisibility ?? privacy.followingVisibility,
    discoverable: discoverable ?? privacy.discoverable,
    dmPolicy: dmPolicy ?? privacy.dmPolicy,
    mentionPolicy: mentionPolicy ?? privacy.mentionPolicy,
  );
}

const List<DropdownMenuItem<ProfileVisibility>> _profileVisibilityItems =
    <DropdownMenuItem<ProfileVisibility>>[
      DropdownMenuItem(value: ProfileVisibility.public, child: Text('所有人')),
      DropdownMenuItem(value: ProfileVisibility.campus, child: Text('校园用户')),
      DropdownMenuItem(value: ProfileVisibility.onlyMe, child: Text('仅自己')),
    ];

const List<DropdownMenuItem<ActivityVisibility>> _activityVisibilityItems =
    <DropdownMenuItem<ActivityVisibility>>[
      DropdownMenuItem(value: ActivityVisibility.public, child: Text('所有人')),
      DropdownMenuItem(value: ActivityVisibility.campus, child: Text('校园用户')),
      DropdownMenuItem(value: ActivityVisibility.onlyMe, child: Text('仅自己')),
    ];

const List<DropdownMenuItem<RelationshipListVisibility>>
_relationshipVisibilityItems = <DropdownMenuItem<RelationshipListVisibility>>[
  DropdownMenuItem(
    value: RelationshipListVisibility.public,
    child: Text('所有人'),
  ),
  DropdownMenuItem(
    value: RelationshipListVisibility.campus,
    child: Text('校园用户'),
  ),
  DropdownMenuItem(
    value: RelationshipListVisibility.followers,
    child: Text('我的关注者'),
  ),
  DropdownMenuItem(
    value: RelationshipListVisibility.onlyMe,
    child: Text('仅自己'),
  ),
];

const List<DropdownMenuItem<DmPolicy>> _dmPolicyItems =
    <DropdownMenuItem<DmPolicy>>[
      DropdownMenuItem(value: DmPolicy.everyone, child: Text('所有人')),
      DropdownMenuItem(value: DmPolicy.following, child: Text('我关注的人')),
      DropdownMenuItem(value: DmPolicy.nobody, child: Text('不允许新私信')),
    ];

const List<DropdownMenuItem<MentionPolicy>> _mentionPolicyItems =
    <DropdownMenuItem<MentionPolicy>>[
      DropdownMenuItem(value: MentionPolicy.everyone, child: Text('所有人')),
      DropdownMenuItem(value: MentionPolicy.following, child: Text('我关注的人')),
      DropdownMenuItem(value: MentionPolicy.nobody, child: Text('不生成提及通知')),
    ];
