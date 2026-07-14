import 'package:flutter/material.dart';

import '../data/captcha_client.dart';

Future<String?> showCaptchaDialog({
  required BuildContext context,
  required CaptchaClient client,
}) {
  return showDialog<String>(
    context: context,
    barrierDismissible: false,
    builder: (BuildContext context) => _CaptchaDialog(client: client),
  );
}

class _CaptchaDialog extends StatefulWidget {
  const _CaptchaDialog({required this.client});

  final CaptchaClient client;

  @override
  State<_CaptchaDialog> createState() => _CaptchaDialogState();
}

class _CaptchaDialogState extends State<_CaptchaDialog> {
  CaptchaChallenge? _challenge;
  Set<int> _selected = <int>{};
  String? _error;
  bool _isLoading = true;
  bool _isVerifying = false;
  int _loadGeneration = 0;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load([String? statusMessage]) async {
    final int generation = ++_loadGeneration;
    setState(() {
      _isLoading = true;
      _challenge = null;
      _selected = <int>{};
      _error = statusMessage;
    });
    try {
      final CaptchaChallenge challenge = await widget.client.loadChallenge();
      if (mounted && generation == _loadGeneration) {
        setState(() => _challenge = challenge);
      }
    } on CaptchaFailure catch (failure) {
      if (mounted && generation == _loadGeneration) {
        setState(() => _error = failure.message);
      }
    } finally {
      if (mounted && generation == _loadGeneration) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _verify() async {
    final CaptchaChallenge? challenge = _challenge;
    if (challenge == null || _isVerifying) {
      return;
    }
    setState(() {
      _isVerifying = true;
      _error = null;
    });
    try {
      final String token = await widget.client.verify(
        challenge: challenge,
        selectedIndices: _selected,
      );
      if (mounted) {
        Navigator.of(context).pop(token);
      }
    } on CaptchaFailure catch (failure) {
      if (mounted) {
        await _load(failure.message);
      }
    } finally {
      if (mounted) {
        setState(() => _isVerifying = false);
      }
    }
  }

  void _toggle(int index) {
    if (_isVerifying) {
      return;
    }
    setState(() {
      final Set<int> next = Set<int>.of(_selected);
      next.contains(index) ? next.remove(index) : next.add(index);
      _selected = next;
    });
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      icon: const Icon(Icons.shield_outlined),
      title: const Text('完成人机验证'),
      content: SizedBox(
        width: 480,
        child: AnimatedSwitcher(
          duration: const Duration(milliseconds: 200),
          child: _content(context),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _isVerifying ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        TextButton(
          onPressed: _isLoading || _isVerifying ? null : _load,
          child: const Text('换一组'),
        ),
        FilledButton.icon(
          onPressed: _challenge == null || _isVerifying ? null : _verify,
          icon: _isVerifying
              ? const SizedBox.square(
                  dimension: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Icon(Icons.verified_user_outlined),
          label: Text(_isVerifying ? '正在验证' : '提交验证'),
        ),
      ],
    );
  }

  Widget _content(BuildContext context) {
    if (_isLoading) {
      return const SizedBox(
        key: ValueKey<String>('captcha-loading'),
        height: 260,
        child: Center(child: CircularProgressIndicator()),
      );
    }
    final CaptchaChallenge? challenge = _challenge;
    if (challenge == null) {
      return SizedBox(
        key: const ValueKey<String>('captcha-error'),
        height: 260,
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            const Icon(Icons.error_outline_rounded, size: 36),
            const SizedBox(height: 12),
            Text(_error ?? '验证码加载失败', textAlign: TextAlign.center),
            const SizedBox(height: 16),
            OutlinedButton(onPressed: _load, child: const Text('重试')),
          ],
        ),
      );
    }
    return Column(
      key: ValueKey<String>(challenge.puzzleToken),
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Text(challenge.prompt),
        const SizedBox(height: 12),
        GridView.builder(
          shrinkWrap: true,
          physics: const NeverScrollableScrollPhysics(),
          gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
            crossAxisCount: 3,
            crossAxisSpacing: 8,
            mainAxisSpacing: 8,
          ),
          itemCount: challenge.imageUris.length,
          itemBuilder: (BuildContext context, int index) {
            final bool isSelected = _selected.contains(index);
            return Semantics(
              button: true,
              selected: isSelected,
              label: '验证码图片选项 ${index + 1}${isSelected ? '，已选择' : ''}',
              child: InkWell(
                onTap: () => _toggle(index),
                borderRadius: BorderRadius.circular(12),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(
                      width: 3,
                      color: isSelected
                          ? Theme.of(context).colorScheme.primary
                          : Theme.of(context).colorScheme.outlineVariant,
                    ),
                  ),
                  child: ClipRRect(
                    borderRadius: BorderRadius.circular(9),
                    child: Stack(
                      fit: StackFit.expand,
                      children: <Widget>[
                        ExcludeSemantics(
                          child: Image.network(
                            challenge.imageUris[index].toString(),
                            fit: BoxFit.cover,
                            errorBuilder:
                                (
                                  BuildContext context,
                                  Object error,
                                  StackTrace? stackTrace,
                                ) => const ColoredBox(
                                  color: Colors.black12,
                                  child: Icon(Icons.broken_image_outlined),
                                ),
                          ),
                        ),
                        if (isSelected)
                          ColoredBox(
                            color: Theme.of(
                              context,
                            ).colorScheme.primary.withValues(alpha: 0.24),
                            child: const Icon(Icons.check_circle_rounded),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            );
          },
        ),
        const SizedBox(height: 10),
        Text('已选择 ${_selected.length} 张；没有符合项时可直接提交。'),
        if (_error case final String error) ...<Widget>[
          const SizedBox(height: 8),
          Text(
            error,
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ],
      ],
    );
  }
}
