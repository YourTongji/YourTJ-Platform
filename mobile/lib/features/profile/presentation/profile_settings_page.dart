import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../account/data/account_repository.dart';
import '../../account/presentation/account_page_layout.dart';
import '../../media/data/media_uploader.dart';
import '../../media/presentation/media_upload_button.dart';

class ProfileSettingsPage extends ConsumerStatefulWidget {
  const ProfileSettingsPage({super.key});

  @override
  ConsumerState<ProfileSettingsPage> createState() =>
      _ProfileSettingsPageState();
}

class _ProfileSettingsPageState extends ConsumerState<ProfileSettingsPage> {
  final GlobalKey<FormState> _formKey = GlobalKey<FormState>();
  final TextEditingController _handleController = TextEditingController();
  final TextEditingController _displayNameController = TextEditingController();
  final TextEditingController _schoolController = TextEditingController();
  final TextEditingController _bioController = TextEditingController();
  final TextEditingController _websiteController = TextEditingController();
  ApiFailure? _failure;
  bool _isLoading = true;
  bool _isSaving = false;
  bool _isUpdatingAsset = false;
  String? _originalHandle;
  String? _pendingAvatarId;
  String? _pendingBannerId;
  MyProfile? _profile;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _handleController.dispose();
    _displayNameController.dispose();
    _schoolController.dispose();
    _bioController.dispose();
    _websiteController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    setState(() {
      _isLoading = true;
      _failure = null;
    });
    try {
      final MyProfile profile = await ref
          .read(accountRepositoryProvider)
          .getMyProfile();
      if (!mounted) {
        return;
      }
      final String handle =
          ref.read(sessionManagerProvider).state.account?.handle ?? '';
      _handleController.text = handle;
      _displayNameController.text = profile.displayName ?? '';
      _schoolController.text = profile.school;
      _bioController.text = profile.bio ?? '';
      _websiteController.text = profile.website ?? '';
      setState(() {
        _profile = profile;
        _originalHandle = handle;
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

  Future<void> _save() async {
    if (_isSaving || !(_formKey.currentState?.validate() ?? false)) {
      return;
    }
    setState(() {
      _isSaving = true;
      _failure = null;
    });
    bool profileWasSaved = false;
    try {
      final AccountRepository repository = ref.read(accountRepositoryProvider);
      final MyProfile profile = await repository.updateMyProfile(
        ProfileUpdateInput(
          displayName: _nullableText(_displayNameController.text),
          school: _schoolController.text.trim(),
          bio: _nullableText(_bioController.text),
          website: _nullableText(_websiteController.text),
        ),
      );
      profileWasSaved = true;
      final String handle = _handleController.text.trim();
      if (handle != _originalHandle) {
        await repository.updateHandle(handle);
        await ref.read(sessionManagerProvider).retrySession();
      }
      if (!mounted) {
        return;
      }
      setState(() {
        _profile = profile;
        _originalHandle = handle;
      });
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('个人资料已与服务器同步')));
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() {
          _failure = ApiFailure(
            kind: failure.kind,
            code: failure.code,
            statusCode: failure.statusCode,
            message: profileWasSaved
                ? '资料文本已保存，但 Handle 更新失败：${failure.message}'
                : failure.message,
          );
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isSaving = false);
      }
    }
  }

  Future<void> _bindAvatar(String assetId) => _updateAsset(
    () => ref.read(accountRepositoryProvider).bindProfileAvatar(assetId),
    successMessage: '头像已绑定',
    onSuccess: () => _pendingAvatarId = null,
  );

  Future<void> _bindBanner(String assetId) => _updateAsset(
    () => ref.read(accountRepositoryProvider).bindProfileBanner(assetId),
    successMessage: '主页背景已绑定',
    onSuccess: () => _pendingBannerId = null,
  );

  Future<void> _updateAsset(
    Future<void> Function() operation, {
    required String successMessage,
    VoidCallback? onSuccess,
  }) async {
    if (_isUpdatingAsset) {
      return;
    }
    setState(() {
      _isUpdatingAsset = true;
      _failure = null;
    });
    try {
      await operation();
      onSuccess?.call();
      if (!mounted) {
        return;
      }
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(successMessage)));
      await _load();
    } on ApiFailure catch (failure) {
      if (mounted) {
        setState(() {
          _failure = ApiFailure(
            kind: failure.kind,
            code: failure.code,
            statusCode: failure.statusCode,
            message: '${failure.message}。若图片仍在处理，请稍后点击“重试绑定”。',
          );
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isUpdatingAsset = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final Widget child;
    if (_isLoading) {
      child = const AppLoadingState(
        title: '正在读取个人资料',
        description: '展示的是服务器上当前的公开资料字段。',
      );
    } else if (_profile == null && _failure != null) {
      child = AccountFailureView(failure: _failure!, onRetry: _load);
    } else {
      child = _buildForm(context);
    }
    return AccountPageLayout(title: '编辑个人资料', child: child);
  }

  Widget _buildForm(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Form(
        key: _formKey,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            const Text('校园邮箱不是公开资料。头像与背景只绑定平台处理后的媒体，不接受任意图片 URL。'),
            const SizedBox(height: 16),
            _ProfileAssetEditor(
              label: '头像',
              currentAssetId: _profile?.avatarAssetId,
              pendingAssetId: _pendingAvatarId,
              usage: MediaUsage.profileAvatar,
              isBusy: _isUpdatingAsset,
              onUploaded: (CompletedMediaUpload upload) {
                setState(() => _pendingAvatarId = upload.uploadId);
                unawaited(_bindAvatar(upload.uploadId));
              },
              onRetry: _pendingAvatarId == null
                  ? null
                  : () => _bindAvatar(_pendingAvatarId!),
              onRemove: _profile?.avatarAssetId == null
                  ? null
                  : () => _updateAsset(
                      ref.read(accountRepositoryProvider).removeProfileAvatar,
                      successMessage: '头像已移除',
                    ),
            ),
            const SizedBox(height: 10),
            _ProfileAssetEditor(
              label: '主页背景',
              currentAssetId: _profile?.bannerAssetId,
              pendingAssetId: _pendingBannerId,
              usage: MediaUsage.profileBanner,
              isBusy: _isUpdatingAsset,
              onUploaded: (CompletedMediaUpload upload) {
                setState(() => _pendingBannerId = upload.uploadId);
                unawaited(_bindBanner(upload.uploadId));
              },
              onRetry: _pendingBannerId == null
                  ? null
                  : () => _bindBanner(_pendingBannerId!),
              onRemove: _profile?.bannerAssetId == null
                  ? null
                  : () => _updateAsset(
                      ref.read(accountRepositoryProvider).removeProfileBanner,
                      successMessage: '主页背景已移除',
                    ),
            ),
            const SizedBox(height: 24),
            TextFormField(
              controller: _handleController,
              enabled: !_isSaving,
              autocorrect: false,
              decoration: const InputDecoration(
                labelText: '公开 Handle',
                prefixText: '@',
                helperText: '更名会受冷却期与保留名检查约束。',
              ),
              validator: (String? value) {
                final String handle = value?.trim() ?? '';
                if (!RegExp(r'^[a-z0-9._-]{3,30}$').hasMatch(handle)) {
                  return '请输入 3–30 位符合规则的 handle';
                }
                return null;
              },
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _displayNameController,
              enabled: !_isSaving,
              maxLength: 50,
              decoration: const InputDecoration(labelText: '显示名（可选）'),
            ),
            const SizedBox(height: 8),
            TextFormField(
              controller: _schoolController,
              enabled: !_isSaving,
              maxLength: 100,
              decoration: const InputDecoration(labelText: '学校'),
              validator: (String? value) {
                final String school = value?.trim() ?? '';
                if (school.isEmpty) {
                  return '学校不能为空';
                }
                return null;
              },
            ),
            const SizedBox(height: 8),
            TextFormField(
              controller: _bioController,
              enabled: !_isSaving,
              minLines: 3,
              maxLines: 7,
              maxLength: 500,
              decoration: const InputDecoration(labelText: '个人简介（可选）'),
            ),
            const SizedBox(height: 8),
            TextFormField(
              controller: _websiteController,
              enabled: !_isSaving,
              keyboardType: TextInputType.url,
              decoration: const InputDecoration(
                labelText: '个人网站（可选）',
                hintText: 'https://example.com',
              ),
              validator: _validateWebsite,
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
                  : const Icon(Icons.save_outlined),
              label: Text(_isSaving ? '正在保存' : '保存资料'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ProfileAssetEditor extends StatelessWidget {
  const _ProfileAssetEditor({
    required this.label,
    required this.currentAssetId,
    required this.pendingAssetId,
    required this.usage,
    required this.isBusy,
    required this.onUploaded,
    required this.onRetry,
    required this.onRemove,
  });

  final String label;
  final String? currentAssetId;
  final String? pendingAssetId;
  final MediaUsage usage;
  final bool isBusy;
  final ValueChanged<CompletedMediaUpload> onUploaded;
  final VoidCallback? onRetry;
  final VoidCallback? onRemove;

  @override
  Widget build(BuildContext context) {
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(label, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 4),
            Text(currentAssetId == null ? '尚未绑定' : '当前资源 ID：$currentAssetId'),
            if (pendingAssetId != null)
              Text('待绑定资源 ID：$pendingAssetId（可能仍在平台处理中）'),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                IgnorePointer(
                  ignoring: isBusy,
                  child: MediaUploadButton(
                    kind: MediaUploadKind.image,
                    usage: usage,
                    label: '上传新$label',
                    onUploaded: onUploaded,
                  ),
                ),
                if (onRetry != null)
                  FilledButton.tonal(
                    onPressed: isBusy ? null : onRetry,
                    child: const Text('重试绑定'),
                  ),
                if (onRemove != null)
                  TextButton(
                    onPressed: isBusy ? null : onRemove,
                    child: Text('移除$label'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

String? _validateWebsite(String? value) {
  final String text = value?.trim() ?? '';
  if (text.isEmpty) {
    return null;
  }
  final Uri? uri = Uri.tryParse(text);
  if (uri == null || uri.scheme != 'https' || uri.host.isEmpty) {
    return '网站必须是完整的 HTTPS 链接';
  }
  if (text.length > 2048) {
    return '网站链接过长';
  }
  return null;
}

String? _nullableText(String value) {
  final String trimmed = value.trim();
  return trimmed.isEmpty ? null : trimmed;
}
