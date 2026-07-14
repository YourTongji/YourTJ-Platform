import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../core/network/api_failure.dart';
import '../../../core/widgets/app_state_views.dart';
import '../../forum/presentation/forum_widgets.dart';
import '../domain/profile_activity_controller.dart';

class ProfileActivitySection extends StatelessWidget {
  const ProfileActivitySection({required this.controller, super.key});

  final ProfileActivityController controller;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: controller,
      builder: (BuildContext context, Widget? child) {
        if (!controller.canViewActivity) {
          return const AppPermissionState(
            title: '活动列表未公开',
            description: '该用户限制了主页上的主题、回复、媒体与喜欢列表；公开内容仍可在对应板块和主题中查看。',
          );
        }
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            Semantics(
              header: true,
              child: Text(
                '用户动态',
                style: Theme.of(
                  context,
                ).textTheme.titleLarge?.copyWith(fontWeight: FontWeight.w700),
              ),
            ),
            const SizedBox(height: 10),
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: ProfileActivityTab.values
                    .map(
                      (ProfileActivityTab tab) => Padding(
                        padding: const EdgeInsets.only(right: 8),
                        child: ChoiceChip(
                          key: Key('profile-activity-${tab.name}'),
                          selected: controller.selectedTab == tab,
                          onSelected: (_) => controller.selectTab(tab),
                          label: Text(tab.label),
                        ),
                      ),
                    )
                    .toList(growable: false),
              ),
            ),
            const SizedBox(height: 12),
            _selectedContent(context),
          ],
        );
      },
    );
  }

  Widget _selectedContent(BuildContext context) {
    return switch (controller.selectedTab) {
      ProfileActivityTab.threads => _ActivityList<UserThread>(
        key: const ValueKey<String>('profile-threads'),
        state: controller.threads,
        emptyTitle: '暂无公开主题',
        emptyDescription: '该用户还没有发布可见主题。',
        loadingTitle: '正在加载公开主题',
        loadMoreLabel: '加载更多主题',
        onRetry: () =>
            controller.load(ProfileActivityTab.threads, refresh: true),
        onLoadMore: () => controller.loadMore(ProfileActivityTab.threads),
        itemBuilder: (UserThread thread) => _ProfileActivityCard(
          title: thread.title,
          body: thread.bodyExcerpt,
          contentFormat: thread.contentFormat,
          boardSlug: thread.boardSlug,
          attachments: thread.attachments,
          replyCount: thread.replyCount,
          voteCount: thread.voteCount,
          isBookmarked: thread.isBookmarked,
          timestampLabel: formatForumTime(thread.createdAt),
          onOpen: () => context.push(_threadLocation(thread.id)),
          onRefreshDelivery: () =>
              controller.load(ProfileActivityTab.threads, refresh: true),
        ),
      ),
      ProfileActivityTab.comments => _ActivityList<UserComment>(
        key: const ValueKey<String>('profile-comments'),
        state: controller.comments,
        emptyTitle: '暂无公开回复',
        emptyDescription: '该用户还没有发布可见回复。',
        loadingTitle: '正在加载公开回复',
        loadMoreLabel: '加载更多回复',
        onRetry: () =>
            controller.load(ProfileActivityTab.comments, refresh: true),
        onLoadMore: () => controller.loadMore(ProfileActivityTab.comments),
        itemBuilder: (UserComment comment) => _ProfileActivityCard(
          title: comment.threadTitle,
          body: comment.body,
          contentFormat: comment.contentFormat,
          attachments: comment.attachments,
          replyCount: comment.replyCount,
          voteCount: comment.voteCount,
          isBookmarked: comment.isBookmarked,
          timestampLabel: formatForumTime(comment.createdAt),
          onOpen: () => context.push(_threadLocation(comment.threadId)),
          onRefreshDelivery: () =>
              controller.load(ProfileActivityTab.comments, refresh: true),
        ),
      ),
      ProfileActivityTab.media => _ActivityList<ProfileContent>(
        key: const ValueKey<String>('profile-media'),
        state: controller.media,
        emptyTitle: '暂无公开媒体',
        emptyDescription: '该用户还没有发布带有可见图片的主题或回复。',
        loadingTitle: '正在加载公开媒体',
        loadMoreLabel: '加载更多媒体',
        onRetry: () => controller.load(ProfileActivityTab.media, refresh: true),
        onLoadMore: () => controller.loadMore(ProfileActivityTab.media),
        itemBuilder: (ProfileContent content) =>
            _contentCard(context, content, ProfileActivityTab.media),
      ),
      ProfileActivityTab.likes => _ActivityList<ProfileContent>(
        key: const ValueKey<String>('profile-likes'),
        state: controller.likes,
        emptyTitle: '暂无公开喜欢',
        emptyDescription: '该用户还没有点赞当前可见的主题或回复。',
        loadingTitle: '正在加载公开喜欢',
        loadMoreLabel: '加载更多喜欢',
        onRetry: () => controller.load(ProfileActivityTab.likes, refresh: true),
        onLoadMore: () => controller.loadMore(ProfileActivityTab.likes),
        itemBuilder: (ProfileContent content) =>
            _contentCard(context, content, ProfileActivityTab.likes),
      ),
    };
  }

  Widget _contentCard(
    BuildContext context,
    ProfileContent content,
    ProfileActivityTab tab,
  ) {
    final String kind = switch (content.targetType) {
      ProfileContentTargetTypeEnum.thread => '主题',
      ProfileContentTargetTypeEnum.comment => '回复',
      ProfileContentTargetTypeEnum.unknownDefaultOpenApi => '内容',
    };
    return _ProfileActivityCard(
      title: content.title,
      body: content.body,
      contentFormat: content.contentFormat,
      boardSlug: content.boardSlug,
      attachments: content.attachments,
      replyCount: content.replyCount,
      voteCount: content.voteCount,
      isBookmarked: content.isBookmarked,
      timestampLabel: tab == ProfileActivityTab.likes
          ? '喜欢于 ${formatForumTime(content.activityAt)} · $kind'
          : '${formatForumTime(content.createdAt)} · $kind',
      onOpen: () => context.push(_threadLocation(content.threadId)),
      onRefreshDelivery: () => controller.load(tab, refresh: true),
    );
  }
}

String _threadLocation(String threadId) =>
    '/forum/threads/${Uri.encodeComponent(threadId)}';

class _ActivityList<T> extends StatelessWidget {
  const _ActivityList({
    required this.state,
    required this.emptyTitle,
    required this.emptyDescription,
    required this.loadingTitle,
    required this.loadMoreLabel,
    required this.onRetry,
    required this.onLoadMore,
    required this.itemBuilder,
    super.key,
  });

  final ProfileActivityListState<T> state;
  final String emptyTitle;
  final String emptyDescription;
  final String loadingTitle;
  final String loadMoreLabel;
  final VoidCallback onRetry;
  final VoidCallback onLoadMore;
  final Widget Function(T item) itemBuilder;

  @override
  Widget build(BuildContext context) {
    if (state.isLoading && state.items.isEmpty) {
      return AppLoadingState(title: loadingTitle);
    }
    final ApiFailure? failure = state.failure;
    if (failure != null && state.items.isEmpty) {
      if (failure.kind == ApiFailureKind.forbidden ||
          failure.kind == ApiFailureKind.notFound) {
        return AppPermissionState(
          title: '活动列表不可见',
          description: failure.message,
        );
      }
      return AppErrorState(description: failure.message, onRetry: onRetry);
    }
    if (state.hasLoaded && state.items.isEmpty) {
      return AppEmptyState(title: emptyTitle, description: emptyDescription);
    }
    if (!state.hasLoaded && !state.isLoading) {
      return AppErrorState(
        title: '动态尚未加载',
        description: '请点击重试读取当前分页。',
        onRetry: onRetry,
      );
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        if (state.isLoading) const LinearProgressIndicator(),
        ...state.items.map(
          (T item) => Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: itemBuilder(item),
          ),
        ),
        if (failure != null)
          Card(
            child: ListTile(
              leading: const Icon(Icons.sync_problem_rounded),
              title: const Text('动态加载失败'),
              subtitle: Text(failure.message),
              trailing: TextButton(
                onPressed: onRetry,
                child: const Text('刷新重试'),
              ),
            ),
          )
        else if (state.hasMore)
          OutlinedButton.icon(
            onPressed: state.isLoadingMore ? null : onLoadMore,
            icon: state.isLoadingMore
                ? const SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.expand_more_rounded),
            label: Text(state.isLoadingMore ? '加载中' : loadMoreLabel),
          ),
      ],
    );
  }
}

class _ProfileActivityCard extends StatelessWidget {
  const _ProfileActivityCard({
    required this.title,
    required this.body,
    required this.contentFormat,
    required this.attachments,
    required this.replyCount,
    required this.voteCount,
    required this.isBookmarked,
    required this.timestampLabel,
    required this.onOpen,
    required this.onRefreshDelivery,
    this.boardSlug,
  });

  final String title;
  final String? body;
  final ContentFormat contentFormat;
  final String? boardSlug;
  final List<ForumAttachment> attachments;
  final int replyCount;
  final int voteCount;
  final bool isBookmarked;
  final String timestampLabel;
  final VoidCallback onOpen;
  final VoidCallback onRefreshDelivery;

  @override
  Widget build(BuildContext context) {
    final String visibleTitle = title.trim().isEmpty ? '查看所在主题' : title;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onOpen,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: Text(
                      visibleTitle,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ),
                  if (isBookmarked)
                    const Icon(Icons.bookmark_rounded, semanticLabel: '已收藏'),
                ],
              ),
              const SizedBox(height: 6),
              Wrap(
                spacing: 8,
                runSpacing: 4,
                children: <Widget>[
                  Text(timestampLabel),
                  if (boardSlug?.trim().isNotEmpty == true)
                    Chip(
                      visualDensity: VisualDensity.compact,
                      label: Text(boardSlug!),
                    ),
                ],
              ),
              if (body?.trim().isNotEmpty == true) ...<Widget>[
                const SizedBox(height: 10),
                ForumBody(
                  source: body!,
                  format: contentFormat,
                  attachments: attachments,
                  onRefreshDelivery: onRefreshDelivery,
                ),
              ],
              if (attachments.isNotEmpty) ...<Widget>[
                const SizedBox(height: 10),
                ForumAttachmentImage(
                  attachment: attachments.first,
                  onRefreshDelivery: onRefreshDelivery,
                ),
              ],
              const SizedBox(height: 12),
              Row(
                children: <Widget>[
                  const Icon(Icons.arrow_upward_rounded, size: 18),
                  const SizedBox(width: 4),
                  Text('$voteCount'),
                  const SizedBox(width: 18),
                  const Icon(Icons.chat_bubble_outline_rounded, size: 18),
                  const SizedBox(width: 4),
                  Text('$replyCount'),
                  const Spacer(),
                  const Icon(Icons.chevron_right_rounded),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
