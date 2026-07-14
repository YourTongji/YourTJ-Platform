import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/core/network/api_failure.dart';
import 'package:yourtj_mobile/features/profile/data/profile_activity_repository.dart';
import 'package:yourtj_mobile/features/profile/domain/profile_activity_controller.dart';

void main() {
  test('loads tabs lazily and deduplicates stable pagination', () async {
    final _FakeProfileActivitySource source = _FakeProfileActivitySource();
    source.onThreads = (String handle, String? cursor) async {
      if (cursor == null) {
        return ProfileActivityPage<UserThread>(
          items: <UserThread>[_thread('thread-1')],
          nextCursor: 'next',
          hasMore: true,
        );
      }
      return ProfileActivityPage<UserThread>(
        items: <UserThread>[_thread('thread-1'), _thread('thread-2')],
        nextCursor: null,
        hasMore: false,
      );
    };
    source.onComments = (String handle, String? cursor) async =>
        ProfileActivityPage<UserComment>(
          items: <UserComment>[_comment('comment-1')],
          nextCursor: null,
          hasMore: false,
        );
    final ProfileActivityController controller = ProfileActivityController(
      source,
    );
    addTearDown(controller.dispose);
    controller.configure(
      handle: 'Alice',
      viewerKey: 'anonymous',
      canViewActivity: true,
    );

    await controller.loadSelected();
    await controller.loadMore(ProfileActivityTab.threads);

    expect(source.threadCalls, <String?>[null, 'next']);
    expect(controller.threads.items.map((UserThread item) => item.id), <String>[
      'thread-1',
      'thread-2',
    ]);
    expect(source.commentCalls, isEmpty);

    controller.selectTab(ProfileActivityTab.comments);
    await _waitUntil(() => controller.comments.hasLoaded);

    expect(source.commentCalls, <String?>[null]);
    expect(controller.comments.items.single.id, 'comment-1');
  });

  test('handle changes discard late responses', () async {
    final Completer<ProfileActivityPage<UserThread>> oldRequest =
        Completer<ProfileActivityPage<UserThread>>();
    final Completer<ProfileActivityPage<UserThread>> currentRequest =
        Completer<ProfileActivityPage<UserThread>>();
    final _FakeProfileActivitySource source = _FakeProfileActivitySource()
      ..onThreads = (String handle, String? cursor) =>
          handle == 'bob' ? oldRequest.future : currentRequest.future;
    final ProfileActivityController controller = ProfileActivityController(
      source,
    );
    addTearDown(controller.dispose);
    controller.configure(
      handle: 'bob',
      viewerKey: 'viewer-1',
      canViewActivity: true,
    );
    final Future<void> staleLoad = controller.loadSelected();

    controller.configure(
      handle: 'alice',
      viewerKey: 'viewer-1',
      canViewActivity: true,
    );
    final Future<void> currentLoad = controller.loadSelected();
    currentRequest.complete(
      ProfileActivityPage<UserThread>(
        items: <UserThread>[_thread('alice-thread')],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await currentLoad;
    oldRequest.complete(
      ProfileActivityPage<UserThread>(
        items: <UserThread>[_thread('bob-thread')],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await staleLoad;

    expect(controller.handle, 'alice');
    expect(controller.threads.items.single.id, 'alice-thread');
  });

  test('viewer changes discard late responses for the same handle', () async {
    final Completer<ProfileActivityPage<UserThread>> oldRequest =
        Completer<ProfileActivityPage<UserThread>>();
    final Completer<ProfileActivityPage<UserThread>> currentRequest =
        Completer<ProfileActivityPage<UserThread>>();
    int call = 0;
    final _FakeProfileActivitySource source = _FakeProfileActivitySource()
      ..onThreads = (String handle, String? cursor) {
        call += 1;
        return call == 1 ? oldRequest.future : currentRequest.future;
      };
    final ProfileActivityController controller = ProfileActivityController(
      source,
    );
    addTearDown(controller.dispose);
    controller.configure(
      handle: 'alice',
      viewerKey: 'viewer-1',
      canViewActivity: true,
    );
    final Future<void> staleLoad = controller.loadSelected();

    controller.configure(
      handle: 'alice',
      viewerKey: 'viewer-2',
      canViewActivity: true,
    );
    final Future<void> currentLoad = controller.loadSelected();
    currentRequest.complete(
      ProfileActivityPage<UserThread>(
        items: <UserThread>[_thread('viewer-2-thread')],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await currentLoad;
    oldRequest.complete(
      ProfileActivityPage<UserThread>(
        items: <UserThread>[_thread('viewer-1-thread')],
        nextCursor: null,
        hasMore: false,
      ),
    );
    await staleLoad;

    expect(controller.threads.items.single.id, 'viewer-2-thread');
  });

  test('private activity never calls protected endpoints', () async {
    final _FakeProfileActivitySource source = _FakeProfileActivitySource();
    final ProfileActivityController controller = ProfileActivityController(
      source,
    );
    addTearDown(controller.dispose);
    controller.configure(
      handle: 'alice',
      viewerKey: 'anonymous',
      canViewActivity: false,
    );

    await controller.loadSelected();
    controller.selectTab(ProfileActivityTab.likes);
    await Future<void>.delayed(Duration.zero);

    expect(source.threadCalls, isEmpty);
    expect(source.likesCalls, isEmpty);
  });

  test(
    'load-more failure preserves canonical items and retry cursor',
    () async {
      final _FakeProfileActivitySource source = _FakeProfileActivitySource();
      source.onThreads = (String handle, String? cursor) async {
        if (cursor != null) {
          throw const ApiFailure(
            kind: ApiFailureKind.offline,
            message: '网络不可用',
          );
        }
        return ProfileActivityPage<UserThread>(
          items: <UserThread>[_thread('thread-1')],
          nextCursor: 'next',
          hasMore: true,
        );
      };
      final ProfileActivityController controller = ProfileActivityController(
        source,
      );
      addTearDown(controller.dispose);
      controller.configure(
        handle: 'alice',
        viewerKey: 'viewer',
        canViewActivity: true,
      );
      await controller.loadSelected();

      await controller.loadMore(ProfileActivityTab.threads);

      expect(controller.threads.items.single.id, 'thread-1');
      expect(controller.threads.nextCursor, 'next');
      expect(controller.threads.hasMore, isTrue);
      expect(controller.threads.failure?.kind, ApiFailureKind.offline);
    },
  );
}

class _FakeProfileActivitySource implements ProfileActivitySource {
  Future<ProfileActivityPage<UserThread>> Function(String, String?)? onThreads;
  Future<ProfileActivityPage<UserComment>> Function(String, String?)?
  onComments;
  Future<ProfileActivityPage<ProfileContent>> Function(String, String?)?
  onMedia;
  Future<ProfileActivityPage<ProfileContent>> Function(String, String?)?
  onLikes;

  final List<String?> threadCalls = <String?>[];
  final List<String?> commentCalls = <String?>[];
  final List<String?> mediaCalls = <String?>[];
  final List<String?> likesCalls = <String?>[];

  @override
  Future<ProfileActivityPage<UserThread>> threads({
    required String handle,
    String? cursor,
  }) {
    threadCalls.add(cursor);
    return onThreads?.call(handle, cursor) ??
        Future<ProfileActivityPage<UserThread>>.value(
          const ProfileActivityPage<UserThread>(
            items: <UserThread>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }

  @override
  Future<ProfileActivityPage<UserComment>> comments({
    required String handle,
    String? cursor,
  }) {
    commentCalls.add(cursor);
    return onComments?.call(handle, cursor) ??
        Future<ProfileActivityPage<UserComment>>.value(
          const ProfileActivityPage<UserComment>(
            items: <UserComment>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }

  @override
  Future<ProfileActivityPage<ProfileContent>> media({
    required String handle,
    String? cursor,
  }) {
    mediaCalls.add(cursor);
    return onMedia?.call(handle, cursor) ??
        Future<ProfileActivityPage<ProfileContent>>.value(
          const ProfileActivityPage<ProfileContent>(
            items: <ProfileContent>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }

  @override
  Future<ProfileActivityPage<ProfileContent>> likes({
    required String handle,
    String? cursor,
  }) {
    likesCalls.add(cursor);
    return onLikes?.call(handle, cursor) ??
        Future<ProfileActivityPage<ProfileContent>>.value(
          const ProfileActivityPage<ProfileContent>(
            items: <ProfileContent>[],
            nextCursor: null,
            hasMore: false,
          ),
        );
  }
}

Future<void> _waitUntil(bool Function() predicate) async {
  for (int attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) {
      return;
    }
    await Future<void>.delayed(Duration.zero);
  }
  fail('condition did not become true');
}

UserThread _thread(String id) => UserThread(
  id: id,
  title: id,
  bodyExcerpt: '主题摘要',
  contentFormat: ContentFormat.plainV1,
  boardSlug: 'campus',
  replyCount: 1,
  voteCount: 2,
  viewerVote: null,
  isBookmarked: false,
  attachments: const <ForumAttachment>[],
  createdAt: 100,
);

UserComment _comment(String id) => UserComment(
  id: id,
  threadId: 'thread-1',
  threadTitle: '主题标题',
  body: '回复正文',
  contentFormat: ContentFormat.plainV1,
  replyCount: 0,
  voteCount: 1,
  viewerVote: null,
  isBookmarked: false,
  attachments: const <ForumAttachment>[],
  createdAt: 100,
);
