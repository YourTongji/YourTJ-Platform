import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/widgets/platform_avatar.dart';

class ReviewCard extends StatelessWidget {
  const ReviewCard({
    required this.review,
    required this.isBusy,
    required this.onLike,
    required this.onEdit,
    required this.onReport,
    required this.onRefreshAvatar,
    super.key,
  });

  final Review review;
  final bool isBusy;
  final Future<void> Function() onLike;
  final Future<void> Function() onEdit;
  final Future<void> Function() onReport;
  final VoidCallback onRefreshAvatar;

  @override
  Widget build(BuildContext context) {
    final String handle = review.authorHandle?.trim().isNotEmpty == true
        ? review.authorHandle!
        : '匿名用户';
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                PlatformAvatar(
                  compatibilityUrl: review.authorAvatar,
                  fallbackText: handle,
                  semanticLabel: '$handle 的头像',
                  onRefresh: onRefreshAvatar,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Wrap(
                        spacing: 8,
                        runSpacing: 6,
                        crossAxisAlignment: WrapCrossAlignment.center,
                        children: <Widget>[
                          Text(
                            handle,
                            style: Theme.of(context).textTheme.titleSmall,
                          ),
                          _Stars(value: review.rating ?? 0),
                          if (review.semester?.trim().isNotEmpty == true)
                            _ReviewPill(label: review.semester!),
                        ],
                      ),
                      const SizedBox(height: 3),
                      Text(
                        _formatTime(review.createdAt),
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
                _ReviewPill(label: _statusLabel(review.status)),
              ],
            ),
            const SizedBox(height: 14),
            Text(
              review.comment?.trim().isNotEmpty == true
                  ? review.comment!
                  : '这条点评没有正文。',
            ),
            if (review.score?.trim().isNotEmpty == true) ...<Widget>[
              const SizedBox(height: 8),
              Text(
                '成绩：${review.score}',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ],
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              children: <Widget>[
                if (review.canEdit)
                  TextButton.icon(
                    onPressed: isBusy ? null : onEdit,
                    icon: const Icon(Icons.edit_outlined),
                    label: const Text('编辑'),
                  )
                else
                  TextButton.icon(
                    onPressed: isBusy ? null : onLike,
                    icon: Icon(
                      review.viewerLiked
                          ? Icons.favorite_rounded
                          : Icons.favorite_border_rounded,
                    ),
                    label: Text(
                      review.viewerLiked
                          ? '${review.approveCount ?? 0} 已赞同'
                          : '${review.approveCount ?? 0} 赞同',
                    ),
                  ),
                if (review.canReport)
                  TextButton.icon(
                    onPressed: isBusy ? null : onReport,
                    icon: const Icon(Icons.flag_outlined),
                    label: const Text('举报'),
                  ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

class ReviewEditDraft {
  const ReviewEditDraft({
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

Future<ReviewEditDraft?> requestReviewEdit(
  BuildContext context,
  Review review,
) {
  final TextEditingController semester = TextEditingController(
    text: review.semester,
  );
  final TextEditingController score = TextEditingController(text: review.score);
  final TextEditingController comment = TextEditingController(
    text: review.comment,
  );
  int rating = review.rating ?? 0;
  return showDialog<ReviewEditDraft>(
    context: context,
    builder: (BuildContext context) {
      return StatefulBuilder(
        builder: (BuildContext context, StateSetter setState) {
          return AlertDialog(
            title: const Text('编辑点评'),
            content: SingleChildScrollView(
              child: SizedBox(
                width: 500,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    Semantics(
                      label: '课程评分，当前 $rating 星',
                      child: Wrap(
                        children: List<Widget>.generate(5, (int index) {
                          final int value = index + 1;
                          return IconButton(
                            tooltip: '$value 星',
                            onPressed: () => setState(() => rating = value),
                            icon: Icon(
                              value <= rating
                                  ? Icons.star_rounded
                                  : Icons.star_outline_rounded,
                            ),
                          );
                        }),
                      ),
                    ),
                    const SizedBox(height: 8),
                    TextField(
                      controller: semester,
                      decoration: const InputDecoration(labelText: '学期（可选）'),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: score,
                      decoration: const InputDecoration(labelText: '成绩（可选）'),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      controller: comment,
                      minLines: 3,
                      maxLines: 8,
                      decoration: const InputDecoration(
                        labelText: '正文（可选）',
                        alignLabelWithHint: true,
                      ),
                    ),
                  ],
                ),
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('取消'),
              ),
              FilledButton(
                onPressed: rating < 1
                    ? null
                    : () => Navigator.of(context).pop(
                        ReviewEditDraft(
                          rating: rating,
                          comment: comment.text.trim(),
                          semester: semester.text.trim(),
                          score: score.text.trim(),
                        ),
                      ),
                child: const Text('保存修改'),
              ),
            ],
          );
        },
      );
    },
  ).whenComplete(() {
    semester.dispose();
    score.dispose();
    comment.dispose();
  });
}

Future<String?> requestReviewReportReason(BuildContext context) {
  final TextEditingController controller = TextEditingController();
  String? error;
  return showDialog<String>(
    context: context,
    builder: (BuildContext context) {
      return StatefulBuilder(
        builder: (BuildContext context, StateSetter setState) {
          return AlertDialog(
            title: const Text('举报点评'),
            content: SizedBox(
              width: 460,
              child: TextField(
                controller: controller,
                autofocus: true,
                minLines: 3,
                maxLines: 6,
                maxLength: 500,
                decoration: InputDecoration(
                  labelText: '举报原因',
                  hintText: '请说明具体问题（3–500 字）',
                  errorText: error,
                  alignLabelWithHint: true,
                ),
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: const Text('取消'),
              ),
              FilledButton(
                onPressed: () {
                  final String reason = controller.text.trim();
                  if (reason.length < 3 || reason.length > 500) {
                    setState(() => error = '请输入 3–500 个字符');
                    return;
                  }
                  Navigator.of(context).pop(reason);
                },
                child: const Text('继续验证'),
              ),
            ],
          );
        },
      );
    },
  ).whenComplete(controller.dispose);
}

class _Stars extends StatelessWidget {
  const _Stars({required this.value});

  final int value;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: '$value 星',
      child: ExcludeSemantics(
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: List<Widget>.generate(5, (int index) {
            return Icon(
              index < value ? Icons.star_rounded : Icons.star_outline_rounded,
              size: 16,
              color: Theme.of(context).colorScheme.primary,
            );
          }),
        ),
      ),
    );
  }
}

class _ReviewPill extends StatelessWidget {
  const _ReviewPill({required this.label});

  final String label;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 4),
        child: Text(label, style: Theme.of(context).textTheme.labelSmall),
      ),
    );
  }
}

String _formatTime(int? seconds) {
  if (seconds == null) {
    return '时间待同步';
  }
  final DateTime date = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  return '${date.year}-${date.month.toString().padLeft(2, '0')}-'
      '${date.day.toString().padLeft(2, '0')}';
}

String _statusLabel(ReviewStatusEnum? status) {
  return switch (status) {
    ReviewStatusEnum.visible || null => '已发布',
    ReviewStatusEnum.pending => '审核中',
    ReviewStatusEnum.hidden => '已隐藏',
    ReviewStatusEnum.unknownDefaultOpenApi => '未知状态',
  };
}
