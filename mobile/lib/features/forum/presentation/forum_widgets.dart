import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/content/platform_content.dart';
import '../../../core/widgets/platform_avatar.dart';

String formatForumTime(int unixSeconds) {
  final DateTime value = DateTime.fromMillisecondsSinceEpoch(
    unixSeconds * 1000,
    isUtc: true,
  ).toLocal();
  final Duration difference = DateTime.now().difference(value);
  if (difference.inMinutes < 1) {
    return '刚刚';
  }
  if (difference.inHours < 1) {
    return '${difference.inMinutes} 分钟前';
  }
  if (difference.inDays < 1) {
    return '${difference.inHours} 小时前';
  }
  if (difference.inDays < 30) {
    return '${difference.inDays} 天前';
  }
  return '${value.year}-${value.month.toString().padLeft(2, '0')}-'
      '${value.day.toString().padLeft(2, '0')}';
}

class ForumThreadCard extends StatelessWidget {
  const ForumThreadCard({
    required this.thread,
    required this.boardName,
    this.onRefreshDelivery,
    this.onOpen,
    super.key,
  });

  final ThreadFeed thread;
  final String? boardName;
  final VoidCallback? onRefreshDelivery;
  final VoidCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    final ForumAttachment? attachment = thread.attachments.firstOrNull;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap:
            onOpen ??
            () => context.push(
              '/forum/threads/${Uri.encodeComponent(thread.id)}',
            ),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Wrap(
                spacing: 8,
                runSpacing: 6,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: <Widget>[
                  PlatformAvatar(
                    radius: 17,
                    delivery: thread.authorAvatar,
                    fallbackText: thread.authorHandle,
                    semanticLabel: '${thread.authorHandle} 的头像',
                    onRefresh: onRefreshDelivery,
                  ),
                  Text(
                    thread.authorDisplayName ?? '@${thread.authorHandle}',
                    style: Theme.of(context).textTheme.labelLarge,
                  ),
                  if (thread.authorDisplayName != null)
                    Text(
                      '@${thread.authorHandle}',
                      style: Theme.of(context).textTheme.bodySmall,
                    ),
                  Text(
                    formatForumTime(thread.lastActivityAt),
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  if (boardName case final String name)
                    Chip(
                      visualDensity: VisualDensity.compact,
                      label: Text(name),
                    ),
                  if ((thread.unreadCount ?? 0) > 0)
                    Chip(
                      visualDensity: VisualDensity.compact,
                      label: Text('${thread.unreadCount} 未读'),
                    ),
                ],
              ),
              const SizedBox(height: 12),
              Text(
                thread.title,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
              ),
              if (thread.bodyExcerpt case final String excerpt) ...<Widget>[
                const SizedBox(height: 8),
                Text(
                  excerpt,
                  maxLines: 3,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.bodyMedium,
                ),
              ],
              if (thread.tags.isNotEmpty) ...<Widget>[
                const SizedBox(height: 10),
                Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: thread.tags
                      .map(
                        (String tag) => Chip(
                          visualDensity: VisualDensity.compact,
                          label: Text('#$tag'),
                        ),
                      )
                      .toList(),
                ),
              ],
              if (attachment != null) ...<Widget>[
                const SizedBox(height: 12),
                ForumAttachmentImage(
                  attachment: attachment,
                  onRefreshDelivery: onRefreshDelivery,
                ),
              ],
              const SizedBox(height: 12),
              Row(
                children: <Widget>[
                  const Icon(Icons.arrow_upward_rounded, size: 18),
                  const SizedBox(width: 4),
                  Text('${thread.voteCount}'),
                  const SizedBox(width: 20),
                  const Icon(Icons.chat_bubble_outline_rounded, size: 18),
                  const SizedBox(width: 4),
                  Text('${thread.replyCount}'),
                  const Spacer(),
                  if (thread.isBookmarked)
                    const Icon(Icons.bookmark_rounded, size: 20),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class ForumAttachmentImage extends StatefulWidget {
  const ForumAttachmentImage({
    required this.attachment,
    this.onRefreshDelivery,
    super.key,
  });

  final ForumAttachment attachment;
  final VoidCallback? onRefreshDelivery;

  @override
  State<ForumAttachmentImage> createState() => _ForumAttachmentImageState();
}

class _ForumAttachmentImageState extends State<ForumAttachmentImage> {
  bool _hasRequestedRefresh = false;

  @override
  void didUpdateWidget(ForumAttachmentImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_forumAttachmentIdentity(oldWidget.attachment) !=
        _forumAttachmentIdentity(widget.attachment)) {
      _hasRequestedRefresh = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final ForumAttachment attachment = widget.attachment;
    final Uri? uri = _safeForumDelivery(attachment.url);
    final bool isExpired =
        attachment.expiresAt <=
        DateTime.now().millisecondsSinceEpoch ~/ 1000 + 30;
    if (uri == null || isExpired) {
      _requestRefreshOnce();
      return OutlinedButton.icon(
        onPressed: widget.onRefreshDelivery,
        icon: const Icon(Icons.refresh_rounded),
        label: const Text('图片链接已失效，刷新后查看'),
      );
    }
    return Semantics(
      image: true,
      label: attachment.alt.isEmpty ? '帖子图片' : attachment.alt,
      child: ClipRRect(
        borderRadius: BorderRadius.circular(12),
        child: Image.network(
          uri.toString(),
          width: double.infinity,
          height: 220,
          fit: BoxFit.cover,
          errorBuilder:
              (BuildContext context, Object error, StackTrace? stackTrace) {
                _requestRefreshOnce();
                return OutlinedButton.icon(
                  onPressed: widget.onRefreshDelivery,
                  icon: const Icon(Icons.broken_image_outlined),
                  label: const Text('图片加载失败，点击刷新'),
                );
              },
        ),
      ),
    );
  }

  void _requestRefreshOnce() {
    if (_hasRequestedRefresh || widget.onRefreshDelivery == null) {
      return;
    }
    _hasRequestedRefresh = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted) {
        widget.onRefreshDelivery?.call();
      }
    });
  }
}

class ForumBody extends StatefulWidget {
  const ForumBody({
    required this.source,
    required this.format,
    required this.attachments,
    this.onRefreshDelivery,
    super.key,
  });

  final String source;
  final ContentFormat format;
  final List<ForumAttachment> attachments;
  final VoidCallback? onRefreshDelivery;

  @override
  State<ForumBody> createState() => _ForumBodyState();
}

class _ForumBodyState extends State<ForumBody> {
  static const int _refreshSkewSeconds = 30;

  bool _hasRequestedRefresh = false;

  @override
  void didUpdateWidget(ForumBody oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_attachmentIdentity(oldWidget.attachments) !=
        _attachmentIdentity(widget.attachments)) {
      _hasRequestedRefresh = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final Map<int, Uri> deliveries = <int, Uri>{};
    final int refreshAt = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    for (final ForumAttachment attachment in widget.attachments) {
      final int? assetId = int.tryParse(attachment.assetId);
      final Uri? uri = _safeForumDelivery(attachment.url);
      final bool needsRefresh =
          assetId == null ||
          assetId <= 0 ||
          uri == null ||
          attachment.expiresAt <= refreshAt + _refreshSkewSeconds;
      if (!needsRefresh) {
        deliveries[assetId] = uri;
      } else if (needsRefresh) {
        _requestRefreshOnce();
      }
    }
    return PlatformContent(
      source: widget.source,
      format: widget.format,
      assetDeliveries: deliveries,
      assetBuilder: (Uri delivery, String? label) => Semantics(
        image: true,
        label: label?.trim().isNotEmpty == true ? label!.trim() : '正文图片',
        child: ClipRRect(
          borderRadius: BorderRadius.circular(12),
          child: Image.network(
            delivery.toString(),
            fit: BoxFit.contain,
            errorBuilder:
                (BuildContext context, Object error, StackTrace? stackTrace) {
                  _requestRefreshOnce();
                  return const _ForumBodyUnavailableImage();
                },
          ),
        ),
      ),
      onInternalLink: (String location) => context.push(location),
    );
  }

  void _requestRefreshOnce() {
    if (_hasRequestedRefresh || widget.onRefreshDelivery == null) {
      return;
    }
    _hasRequestedRefresh = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted) {
        widget.onRefreshDelivery?.call();
      }
    });
  }
}

class _ForumBodyUnavailableImage extends StatelessWidget {
  const _ForumBodyUnavailableImage();

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerHigh,
        borderRadius: BorderRadius.circular(12),
      ),
      child: const Padding(
        padding: EdgeInsets.all(16),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(Icons.broken_image_outlined),
            SizedBox(width: 8),
            Flexible(child: Text('图片加载失败，正在刷新交付地址')),
          ],
        ),
      ),
    );
  }
}

Uri? _safeForumDelivery(String value) {
  final Uri? uri = Uri.tryParse(value.trim());
  if (uri == null ||
      uri.scheme != 'https' ||
      !uri.hasAuthority ||
      uri.host.isEmpty ||
      uri.userInfo.isNotEmpty ||
      uri.hasFragment) {
    return null;
  }
  return uri;
}

String _attachmentIdentity(List<ForumAttachment> attachments) {
  return attachments.map(_forumAttachmentIdentity).join('\u0001');
}

String _forumAttachmentIdentity(ForumAttachment attachment) =>
    '${attachment.assetId}\u0000${attachment.url}\u0000${attachment.expiresAt}';
