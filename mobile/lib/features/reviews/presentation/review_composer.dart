import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:uuid/uuid.dart';

import '../../../app/app_services.dart';
import '../../../app/router.dart';
import '../../../core/network/api_failure.dart';
import '../../auth/domain/session_state.dart';
import '../../captcha/presentation/captcha_dialog.dart';

@immutable
class ReviewDraft {
  const ReviewDraft({
    required this.rating,
    required this.comment,
    required this.semester,
    required this.score,
  });

  final int rating;
  final String comment;
  final String semester;
  final String score;
}

class ReviewComposer extends ConsumerStatefulWidget {
  const ReviewComposer({
    required this.isPublishing,
    required this.onPublish,
    super.key,
  });

  final bool isPublishing;
  final Future<void> Function(
    ReviewDraft draft,
    String captchaToken,
    String idempotencyKey,
  )
  onPublish;

  @override
  ConsumerState<ReviewComposer> createState() => _ReviewComposerState();
}

class _ReviewComposerState extends ConsumerState<ReviewComposer> {
  final TextEditingController _semester = TextEditingController();
  final TextEditingController _score = TextEditingController();
  final TextEditingController _comment = TextEditingController();
  int _rating = 5;
  String? _idempotencyKey;

  @override
  void dispose() {
    _semester.dispose();
    _score.dispose();
    _comment.dispose();
    super.dispose();
  }

  void _changed() {
    _idempotencyKey = null;
  }

  Future<void> _publish() async {
    if (widget.isPublishing) {
      return;
    }
    final String? captchaToken = await showCaptchaDialog(
      context: context,
      client: ref.read(captchaClientProvider),
    );
    if (!mounted || captchaToken == null) {
      return;
    }
    final String idempotencyKey = _idempotencyKey ??=
        'mobile-review-${const Uuid().v4()}';
    final ReviewDraft draft = ReviewDraft(
      rating: _rating,
      comment: _comment.text.trim(),
      semester: _semester.text.trim(),
      score: _score.text.trim(),
    );
    try {
      await widget.onPublish(draft, captchaToken, idempotencyKey);
      if (!mounted) {
        return;
      }
      _comment.clear();
      _semester.clear();
      _score.clear();
      setState(() {
        _rating = 5;
        _idempotencyKey = null;
      });
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('点评已发布')));
    } on ApiFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(
              failure.kind == ApiFailureKind.timeout
                  ? '${failure.message}。请先刷新点评列表确认结果，再使用同一内容重试。'
                  : failure.message,
            ),
          ),
        );
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final AsyncValue<SessionState> asyncSession = ref.watch(
      sessionStateProvider,
    );
    final SessionState session =
        asyncSession.value ?? ref.read(sessionManagerProvider).state;
    if (!session.isAuthenticated) {
      return Card(
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('登录后发布点评', style: Theme.of(context).textTheme.titleMedium),
              const SizedBox(height: 6),
              const Text('点评会绑定公开账号，但不会公开校园邮箱。'),
              const SizedBox(height: 14),
              FilledButton.icon(
                onPressed: () => context.push(
                  publicInteractionLoginLocation(GoRouterState.of(context).uri),
                ),
                icon: const Icon(Icons.login_rounded),
                label: const Text('登录'),
              ),
            ],
          ),
        ),
      );
    }

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(18),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text('写点评', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(
              '评分必填；发布前需要完成人机验证，重复提交由幂等键保护。',
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 14),
            Semantics(
              label: '课程评分，当前 $_rating 星',
              child: Wrap(
                spacing: 2,
                children: List<Widget>.generate(5, (int index) {
                  final int value = index + 1;
                  return IconButton(
                    tooltip: '$value 星',
                    onPressed: widget.isPublishing
                        ? null
                        : () {
                            setState(() {
                              _rating = value;
                              _changed();
                            });
                          },
                    icon: Icon(
                      value <= _rating
                          ? Icons.star_rounded
                          : Icons.star_outline_rounded,
                      color: Theme.of(context).colorScheme.primary,
                    ),
                  );
                }),
              ),
            ),
            const SizedBox(height: 10),
            LayoutBuilder(
              builder: (BuildContext context, BoxConstraints constraints) {
                final Widget semester = TextField(
                  controller: _semester,
                  enabled: !widget.isPublishing,
                  decoration: const InputDecoration(
                    labelText: '学期（可选）',
                    hintText: '如 2025 春',
                  ),
                  onChanged: (_) => _changed(),
                );
                final Widget score = TextField(
                  controller: _score,
                  enabled: !widget.isPublishing,
                  decoration: const InputDecoration(labelText: '成绩（可选）'),
                  onChanged: (_) => _changed(),
                );
                if (constraints.maxWidth < 520) {
                  return Column(
                    children: <Widget>[
                      semester,
                      const SizedBox(height: 12),
                      score,
                    ],
                  );
                }
                return Row(
                  children: <Widget>[
                    Expanded(child: semester),
                    const SizedBox(width: 12),
                    Expanded(child: score),
                  ],
                );
              },
            ),
            const SizedBox(height: 12),
            TextField(
              controller: _comment,
              enabled: !widget.isPublishing,
              minLines: 4,
              maxLines: 8,
              decoration: const InputDecoration(
                labelText: '正文（可选）',
                hintText: '课程体验、作业、考核、老师风格……',
                alignLabelWithHint: true,
              ),
              onChanged: (_) => _changed(),
            ),
            const SizedBox(height: 14),
            FilledButton.icon(
              onPressed: widget.isPublishing ? null : _publish,
              icon: widget.isPublishing
                  ? const SizedBox.square(
                      dimension: 18,
                      child: CircularProgressIndicator(strokeWidth: 2),
                    )
                  : const Icon(Icons.send_rounded),
              label: Text(widget.isPublishing ? '正在发布' : '发布点评'),
            ),
          ],
        ),
      ),
    );
  }
}
