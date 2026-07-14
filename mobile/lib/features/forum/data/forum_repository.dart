import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../../core/network/api_failure.dart';

enum ForumFeed {
  hot('hot', '热门', false),
  newest('new', '最新', false),
  subscriptions('subscriptions', '订阅', true),
  following('following', '关注', true),
  unread('unread', '未读', true);

  const ForumFeed(this.wireName, this.label, this.requiresAuthentication);

  final String wireName;
  final String label;
  final bool requiresAuthentication;
}

class ForumPageSlice<T> {
  const ForumPageSlice({
    required this.items,
    required this.nextCursor,
    required this.hasMore,
  });

  final List<T> items;
  final String? nextCursor;
  final bool hasMore;
}

class ForumRepository {
  ForumRepository(this._api);

  final ForumApi _api;

  Future<List<Board>> boards() =>
      _request(() => _api.forumBoardsGet(), fallbackMessage: '板块列表响应不完整');

  Future<List<Tag>> tags() =>
      _request(() => _api.forumTagsGet(), fallbackMessage: '标签列表响应不完整');

  Future<ForumPageSlice<ThreadFeed>> threads({
    required ForumFeed feed,
    String? boardId,
    String? tag,
    String? cursor,
  }) async {
    final ThreadFeedPage page = await _request(
      () => _api.forumThreadsGet(
        board: boardId,
        tag: tag,
        sort: feed.wireName,
        cursor: cursor,
        limit: 20,
      ),
      fallbackMessage: '主题列表响应不完整',
    );
    return ForumPageSlice<ThreadFeed>(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  Future<ThreadDetail> thread(String id) => _request(
    () => _api.forumThreadsIdGet(id: id),
    fallbackMessage: '主题详情响应不完整',
  );

  Future<ForumPageSlice<Comment>> comments(
    String threadId, {
    String? cursor,
  }) async {
    final CommentPage page = await _request(
      () => _api.forumThreadsIdCommentsGet(
        id: threadId,
        cursor: cursor,
        limit: 50,
      ),
      fallbackMessage: '回复列表响应不完整',
    );
    return ForumPageSlice<Comment>(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  Future<ThreadDetail> createThread(ThreadInput input) => _request(
    () => _api.forumThreadsPost(threadInput: input),
    fallbackMessage: '发布响应不完整',
  );

  Future<ThreadDetail> updateThread(String id, ThreadUpdateInput input) =>
      _request(
        () => _api.forumThreadsIdPatch(id: id, threadUpdateInput: input),
        fallbackMessage: '编辑响应不完整',
      );

  Future<void> deleteThread(String id) =>
      _requestVoid(() => _api.forumThreadsIdDelete(id: id));

  Future<Comment> createComment(String threadId, CommentInput input) =>
      _request(
        () =>
            _api.forumThreadsIdCommentsPost(id: threadId, commentInput: input),
        fallbackMessage: '回复响应不完整',
      );

  Future<Comment> updateComment(String id, CommentUpdateInput input) =>
      _request(
        () => _api.forumCommentsIdPatch(id: id, commentUpdateInput: input),
        fallbackMessage: '编辑回复响应不完整',
      );

  Future<void> deleteComment(String id) =>
      _requestVoid(() => _api.forumCommentsIdDelete(id: id));

  Future<void> markSolved(String id) =>
      _requestVoid(() => _api.forumCommentsIdSolvePost(id: id));

  Future<void> unmarkSolved(String id) =>
      _requestVoid(() => _api.forumCommentsIdSolveDelete(id: id));

  Future<void> vote({
    required String id,
    required String postType,
    required String value,
    required bool remove,
  }) async {
    if (remove) {
      await _request(
        () => _api.forumPostsIdVoteDelete(id: id, postType: postType),
        fallbackMessage: '投票响应不完整',
      );
      return;
    }
    await _request(
      () => _api.forumPostsIdVotePost(
        id: id,
        voteInput: VoteInput(
          value: value == 'up'
              ? VoteInputValueEnum.up
              : VoteInputValueEnum.down,
          postType: postType == 'thread'
              ? VoteInputPostTypeEnum.thread
              : VoteInputPostTypeEnum.comment,
        ),
      ),
      fallbackMessage: '投票响应不完整',
    );
  }

  Future<void> setBookmark({
    required String id,
    required String postType,
    required bool bookmarked,
  }) async {
    if (bookmarked) {
      await _requestVoid(
        () => _api.forumPostsIdBookmarkDelete(id: id, postType: postType),
      );
      return;
    }
    await _requestVoid(
      () => _api.forumPostsIdBookmarkPut(
        id: id,
        bookmarkInput: BookmarkInput(
          postType: postType == 'thread'
              ? BookmarkInputPostTypeEnum.thread
              : BookmarkInputPostTypeEnum.comment,
        ),
      ),
    );
  }

  Future<void> report({
    required String id,
    required String postType,
    required FlagInputReasonEnum reason,
    String? note,
  }) async {
    await _request(
      () => _api.forumPostsIdFlagPost(
        id: id,
        flagInput: FlagInput(
          reason: reason,
          note: note?.trim().isEmpty == true ? null : note?.trim(),
          postType: postType == 'thread'
              ? FlagInputPostTypeEnum.thread
              : FlagInputPostTypeEnum.comment,
        ),
      ),
      fallbackMessage: '举报响应不完整',
    );
  }

  Future<void> setThreadSubscription({
    required String threadId,
    required String level,
  }) async {
    if (level == 'none') {
      await _requestVoid(
        () => _api.forumSubscriptionsDelete(
          unsubscribeInput: UnsubscribeInput(
            targetType: UnsubscribeInputTargetTypeEnum.thread,
            targetId: threadId,
          ),
        ),
      );
      return;
    }
    final SubscriptionInputLevelEnum wireLevel = switch (level) {
      'watching' => SubscriptionInputLevelEnum.watching,
      'tracking' => SubscriptionInputLevelEnum.tracking,
      'muted' => SubscriptionInputLevelEnum.muted,
      _ => throw const ApiFailure(
        kind: ApiFailureKind.invalidInput,
        message: '不支持的订阅级别',
      ),
    };
    await _requestVoid(
      () => _api.forumSubscriptionsPut(
        subscriptionInput: SubscriptionInput(
          targetType: SubscriptionInputTargetTypeEnum.thread,
          targetId: threadId,
          level: wireLevel,
        ),
      ),
    );
  }

  Future<void> votePoll({
    required String pollId,
    required String optionId,
    required bool remove,
  }) async {
    if (remove) {
      await _request(
        () => _api.forumPollsIdVoteDelete(id: pollId, optionId: optionId),
        fallbackMessage: '投票响应不完整',
      );
      return;
    }
    await _request(
      () => _api.forumPollsIdVotePost(
        id: pollId,
        forumPollsIdVotePostRequest: ForumPollsIdVotePostRequest(
          optionId: optionId,
        ),
      ),
      fallbackMessage: '投票响应不完整',
    );
  }

  Future<void> markRead(String threadId) => _requestVoid(
    () => _api.forumThreadsIdReadPost(
      id: threadId,
      readTrackingInput: ReadTrackingInput(),
    ),
  );

  /// Reads an authorized page of snapshots created before thread edits.
  Future<ForumPageSlice<PostRevision>> threadRevisions(
    String threadId, {
    String? cursor,
  }) async {
    final RevisionPage page = await _request(
      () => _api.forumThreadsIdRevisionsGet(
        id: threadId,
        cursor: cursor,
        limit: 20,
      ),
      fallbackMessage: '主题修订历史响应不完整',
    );
    return ForumPageSlice<PostRevision>(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  /// Reads an authorized page of snapshots created before comment edits.
  Future<ForumPageSlice<PostRevision>> commentRevisions(
    String commentId, {
    String? cursor,
  }) async {
    final RevisionPage page = await _request(
      () => _api.forumCommentsIdRevisionsGet(
        id: commentId,
        cursor: cursor,
        limit: 20,
      ),
      fallbackMessage: '回复修订历史响应不完整',
    );
    return ForumPageSlice<PostRevision>(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  Future<DraftOutput?> draft(String key) async {
    try {
      return await _request(
        () => _api.meDraftsDraftKeyGet(draftKey: key),
        fallbackMessage: '云端草稿响应不完整',
      );
    } on ApiFailure catch (failure) {
      if (failure.kind == ApiFailureKind.notFound) {
        return null;
      }
      rethrow;
    }
  }

  Future<DraftOutput> saveDraft(DraftSaveInput input) => _request(
    () => _api.meDraftsPut(draftSaveInput: input),
    fallbackMessage: '保存草稿响应不完整',
  );

  Future<void> deleteDraft(String key) =>
      _requestVoid(() => _api.meDraftsDraftKeyDelete(draftKey: key));

  Future<ForumPageSlice<Bookmark>> bookmarks({String? cursor}) async {
    final BookmarkPage page = await _request(
      () => _api.forumBookmarksGet(cursor: cursor, limit: 20),
      fallbackMessage: '收藏列表响应不完整',
    );
    return ForumPageSlice<Bookmark>(
      items: page.items,
      nextCursor: page.nextCursor,
      hasMore: page.hasMore,
    );
  }

  Future<T> _request<T>(
    Future<Response<T>> Function() operation, {
    required String fallbackMessage,
  }) async {
    try {
      final Response<T> response = await operation();
      final T? data = response.data;
      if (data == null) {
        throw ApiFailure(
          kind: ApiFailureKind.unexpected,
          message: fallbackMessage,
        );
      }
      return data;
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }

  Future<void> _requestVoid(
    Future<Response<dynamic>> Function() operation,
  ) async {
    try {
      await operation();
    } on DioException catch (exception) {
      throw ApiFailure.fromDio(exception);
    }
  }
}

final Provider<ForumRepository> forumRepositoryProvider =
    Provider<ForumRepository>((Ref ref) {
      return ForumRepository(ref.watch(apiProvider).getForumApi());
    });
