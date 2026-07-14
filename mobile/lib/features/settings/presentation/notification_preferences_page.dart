import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';

class NotificationPreferencesPage extends ConsumerStatefulWidget {
  const NotificationPreferencesPage({super.key});

  @override
  ConsumerState<NotificationPreferencesPage> createState() =>
      _NotificationPreferencesPageState();
}

class _NotificationPreferencesPageState
    extends ConsumerState<NotificationPreferencesPage> {
  NotificationPreferences? _preferences;
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
      final NotificationPrefs response = await ref
          .read(accountRepositoryProvider)
          .getNotificationPreferences();
      if (mounted) {
        setState(() => _preferences = response.prefs);
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
    final NotificationPreferences? preferences = _preferences;
    if (preferences == null || _isSaving) {
      return;
    }
    setState(() {
      _isSaving = true;
      _failure = null;
    });
    try {
      final InAppNotificationPrefs inApp = preferences.inApp;
      final NotificationPrefs response = await ref
          .read(accountRepositoryProvider)
          .updateNotificationPreferences(
            NotificationPrefsInput(
              prefs: NotificationPreferencesInput(
                inApp: InAppNotificationPrefsInput(
                  replies: inApp.replies,
                  mentions: inApp.mentions,
                  quotes: inApp.quotes,
                  votes: inApp.votes,
                  badges: inApp.badges,
                  follows: inApp.follows,
                  subscriptions: inApp.subscriptions,
                  directMessages: inApp.directMessages,
                ),
                email: EmailNotificationPrefs(
                  weeklyDigest: preferences.email.weeklyDigest,
                ),
              ),
            ),
          );
      if (!mounted) {
        return;
      }
      setState(() => _preferences = response.prefs);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('通知偏好已保存')));
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

  void _updateInApp(
    InAppNotificationPrefs Function(InAppNotificationPrefs) change,
  ) {
    final NotificationPreferences? current = _preferences;
    if (current == null) {
      return;
    }
    setState(() {
      _preferences = NotificationPreferences(
        inApp: change(current.inApp),
        email: current.email,
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取通知偏好',
        description: '应用内通知与邮件摘要偏好由服务器跨端保存。',
      );
    } else if (_preferences == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildPreferences(_preferences!);
    }
    return AccountPageLayout(title: '通知偏好', child: child);
  }

  Widget _buildPreferences(NotificationPreferences preferences) {
    final InAppNotificationPrefs inApp = preferences.inApp;
    return ListView(
      padding: const EdgeInsets.all(16),
      children: <Widget>[
        const Padding(
          padding: EdgeInsets.fromLTRB(8, 8, 8, 16),
          child: Text('这些选择控制服务器产生的持久通知。系统级后台 push 尚未完成投递闭环，本页不会伪装成已启用系统推送。'),
        ),
        _PreferenceSwitch(
          title: '回复',
          description: '有人回复你的主题或评论。',
          value: inApp.replies,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, replies: value),
          ),
        ),
        _PreferenceSwitch(
          title: '提及',
          description: '有人在公开内容中语义提及你。',
          value: inApp.mentions,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, mentions: value),
          ),
        ),
        _PreferenceSwitch(
          title: '引用',
          description: '有人引用你的内容。',
          value: inApp.quotes,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, quotes: value),
          ),
        ),
        _PreferenceSwitch(
          title: '点赞与投票',
          description: '你的公开内容获得新的正向互动。',
          value: inApp.votes,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, votes: value),
          ),
        ),
        _PreferenceSwitch(
          title: '徽章与成就',
          description: '你获得平台徽章或成就。',
          value: inApp.badges,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, badges: value),
          ),
        ),
        _PreferenceSwitch(
          title: '新关注者',
          description: '有人开始关注你。',
          value: inApp.follows,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, follows: value),
          ),
        ),
        _PreferenceSwitch(
          title: '订阅内容',
          description: '你订阅的板块或主题出现更新。',
          value: inApp.subscriptions,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, subscriptions: value),
          ),
        ),
        _PreferenceSwitch(
          title: '私信',
          description: '新会话请求或已接受会话有新消息。',
          value: inApp.directMessages,
          onChanged: (bool value) => _updateInApp(
            (InAppNotificationPrefs current) =>
                _copyInApp(current, directMessages: value),
          ),
        ),
        Card(
          child: SwitchListTile.adaptive(
            title: const Text('每周邮件摘要'),
            subtitle: const Text('发送到账号绑定的校园邮箱；本客户端不会显示邮箱地址。'),
            value: preferences.email.weeklyDigest,
            onChanged: (bool value) => setState(() {
              _preferences = NotificationPreferences(
                inApp: preferences.inApp,
                email: EmailNotificationPrefs(weeklyDigest: value),
              );
            }),
          ),
        ),
        if (_failure != null) ...<Widget>[
          const SizedBox(height: 16),
          Text(
            _failure!.message,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
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
              : const Icon(Icons.notifications_active_outlined),
          label: Text(_isSaving ? '正在保存' : '保存通知偏好'),
        ),
      ],
    );
  }
}

class _PreferenceSwitch extends StatelessWidget {
  const _PreferenceSwitch({
    required this.title,
    required this.description,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final String description;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: SwitchListTile.adaptive(
        title: Text(title),
        subtitle: Text(description),
        value: value,
        onChanged: onChanged,
      ),
    );
  }
}

InAppNotificationPrefs _copyInApp(
  InAppNotificationPrefs current, {
  bool? replies,
  bool? mentions,
  bool? quotes,
  bool? votes,
  bool? badges,
  bool? follows,
  bool? subscriptions,
  bool? directMessages,
}) {
  return InAppNotificationPrefs(
    replies: replies ?? current.replies,
    mentions: mentions ?? current.mentions,
    quotes: quotes ?? current.quotes,
    votes: votes ?? current.votes,
    badges: badges ?? current.badges,
    follows: follows ?? current.follows,
    subscriptions: subscriptions ?? current.subscriptions,
    directMessages: directMessages ?? current.directMessages,
  );
}
