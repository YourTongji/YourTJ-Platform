import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';

class OnboardingPage extends ConsumerStatefulWidget {
  const OnboardingPage({super.key, this.returnLocation});

  final String? returnLocation;

  @override
  ConsumerState<OnboardingPage> createState() => _OnboardingPageState();
}

class _OnboardingPageState extends ConsumerState<OnboardingPage> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _handleController = TextEditingController();
  final TextEditingController _displayNameController = TextEditingController();
  final TextEditingController _bioController = TextEditingController();
  OnboardingState? _state;
  ApiFailure? _failure;
  bool _isLoading = true;
  bool _isSubmitting = false;
  bool _hasAcceptedTerms = false;
  bool _isDiscoverable = true;
  ProfileVisibility _profileVisibility = ProfileVisibility.campus;
  ActivityVisibility _activityVisibility = ActivityVisibility.campus;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _handleController.dispose();
    _displayNameController.dispose();
    _bioController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final OnboardingState state = await ref
          .read(accountRepositoryProvider)
          .getOnboarding();
      if (!mounted) {
        return;
      }
      _handleController.text = state.handle;
      _displayNameController.text = state.displayName ?? '';
      _bioController.text = state.bio ?? '';
      setState(() {
        _state = state;
        _profileVisibility = _knownProfileVisibility(state.profileVisibility);
        _activityVisibility = _knownActivityVisibility(
          state.activityVisibility,
        );
        _isDiscoverable = state.discoverable;
        _hasAcceptedTerms =
            !state.required_ &&
            state.acceptedTermsVersion == state.currentTermsVersion;
      });
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

  Future<void> _submit() async {
    final OnboardingState? state = _state;
    if (state == null ||
        _isSubmitting ||
        !_hasAcceptedTerms ||
        !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    setState(() {
      _isSubmitting = true;
      _failure = null;
    });
    try {
      final OnboardingState completed = await ref
          .read(accountRepositoryProvider)
          .completeOnboarding(
            OnboardingCompleteInput(
              handle: _handleController.text.trim(),
              displayName: _nullableText(_displayNameController.text),
              bio: _nullableText(_bioController.text),
              profileVisibility: _profileVisibility,
              activityVisibility: _activityVisibility,
              discoverable: _isDiscoverable,
              acceptedTermsVersion: state.currentTermsVersion,
            ),
          );
      if (completed.required_) {
        throw const ApiFailure(
          kind: ApiFailureKind.conflict,
          message: '服务器仍要求完成首次设置，请检查条款版本后重试',
        );
      }
      await ref.read(sessionManagerProvider).retrySession();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('首次设置已保存')));
      context.go(widget.returnLocation ?? AppRoutes.account);
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() => _failure = failure);
      }
    } finally {
      if (mounted) {
        setState(() => _isSubmitting = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在恢复首次设置',
        description: '正在读取服务器保存的进度。',
      );
    } else if (_state == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildForm(context);
    }
    return AccountPageLayout(title: '首次设置', child: child);
  }

  Widget _buildForm(BuildContext context) {
    final OnboardingState state = _state!;
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Semantics(
              header: true,
              child: Text(
                state.required_ ? '完成账号设置' : '首次设置已完成',
                style: Theme.of(context).textTheme.headlineSmall,
              ),
            ),
            const SizedBox(height: 8),
            const Text('这些选择会同步到 Web；校园邮箱始终不会出现在公开资料中。'),
            const SizedBox(height: 24),
            TextFormField(
              controller: _handleController,
              enabled: !_isSubmitting,
              autocorrect: false,
              textCapitalization: TextCapitalization.none,
              decoration: const InputDecoration(
                labelText: '公开 handle',
                helperText: '3–30 位小写字母、数字、点、下划线或连字号',
                prefixText: '@',
              ),
              validator: (String? value) {
                final String handle = value?.trim() ?? '';
                if (!RegExp(r'^[a-z0-9._-]{3,30}$').hasMatch(handle)) {
                  return '请输入符合规则的公开 handle';
                }
                return null;
              },
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _displayNameController,
              enabled: !_isSubmitting,
              decoration: const InputDecoration(labelText: '显示名（可选）'),
              maxLength: 50,
              validator: (String? value) {
                final String text = value?.trim() ?? '';
                if (text.isNotEmpty && text.length > 50) {
                  return '显示名不能超过 50 个字符';
                }
                return null;
              },
            ),
            const SizedBox(height: 8),
            TextFormField(
              controller: _bioController,
              enabled: !_isSubmitting,
              minLines: 3,
              maxLines: 6,
              maxLength: 500,
              decoration: const InputDecoration(labelText: '个人简介（可选）'),
            ),
            const SizedBox(height: 16),
            DropdownButtonFormField<ProfileVisibility>(
              initialValue: _profileVisibility,
              decoration: const InputDecoration(labelText: '资料可见性'),
              items: _profileVisibilityItems,
              onChanged: _isSubmitting
                  ? null
                  : (ProfileVisibility? value) {
                      if (value != null) {
                        setState(() => _profileVisibility = value);
                      }
                    },
            ),
            const SizedBox(height: 16),
            DropdownButtonFormField<ActivityVisibility>(
              initialValue: _activityVisibility,
              decoration: const InputDecoration(labelText: '个人主页活动列表'),
              items: _activityVisibilityItems,
              onChanged: _isSubmitting
                  ? null
                  : (ActivityVisibility? value) {
                      if (value != null) {
                        setState(() => _activityVisibility = value);
                      }
                    },
            ),
            const SizedBox(height: 12),
            SwitchListTile.adaptive(
              contentPadding: EdgeInsets.zero,
              title: const Text('允许被搜索和发现'),
              subtitle: const Text('关闭后，已知 handle 的可见访问仍受上述权限控制。'),
              value: _isDiscoverable,
              onChanged: _isSubmitting
                  ? null
                  : (bool value) => setState(() => _isDiscoverable = value),
            ),
            const SizedBox(height: 16),
            Card(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Text(
                      '当前条款版本：${state.currentTermsVersion}',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      '请确认社区规则、隐私说明与积分闭环边界。积分不充值、不提现、不与法币兑换，也不支持无理由自由转账。',
                    ),
                    const SizedBox(height: 8),
                    CheckboxListTile(
                      contentPadding: EdgeInsets.zero,
                      controlAffinity: ListTileControlAffinity.leading,
                      title: const Text('我已阅读并同意当前必要条款'),
                      value: _hasAcceptedTerms,
                      onChanged: _isSubmitting
                          ? null
                          : (bool? value) {
                              setState(
                                () => _hasAcceptedTerms = value ?? false,
                              );
                            },
                    ),
                  ],
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
              onPressed: _isSubmitting || !_hasAcceptedTerms ? null : _submit,
              icon: _isSubmitting
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.check_circle_outline_rounded),
              label: Text(_isSubmitting ? '正在保存' : '保存并继续'),
            ),
          ],
        ),
      ),
    );
  }
}

String? _nullableText(String value) {
  final String trimmed = value.trim();
  return trimmed.isEmpty ? null : trimmed;
}

ProfileVisibility _knownProfileVisibility(ProfileVisibility value) {
  return switch (value) {
    ProfileVisibility.public ||
    ProfileVisibility.campus ||
    ProfileVisibility.onlyMe => value,
    ProfileVisibility.unknownDefaultOpenApi => ProfileVisibility.onlyMe,
  };
}

ActivityVisibility _knownActivityVisibility(ActivityVisibility value) {
  return switch (value) {
    ActivityVisibility.public ||
    ActivityVisibility.campus ||
    ActivityVisibility.onlyMe => value,
    ActivityVisibility.unknownDefaultOpenApi => ActivityVisibility.onlyMe,
  };
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
