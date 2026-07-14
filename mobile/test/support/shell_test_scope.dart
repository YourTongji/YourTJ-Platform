import 'package:dio/dio.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/forum/data/forum_repository.dart';
import 'package:yourtj_mobile/features/home/data/home_repository.dart';
import 'package:yourtj_mobile/features/messages/domain/message_badge_counts.dart';

Widget shellTestScope({
  required Widget child,
  MessageBadgeCounts messageBadges = MessageBadgeCounts.zero,
  SessionState sessionState = const SessionState.anonymous(generation: 1),
}) {
  return ProviderScope(
    overrides: [
      forumRepositoryProvider.overrideWithValue(_EmptyForumRepository()),
      homeRepositoryProvider.overrideWithValue(_EmptyHomeRepository()),
      sessionStateProvider.overrideWith(
        (Ref ref) => Stream<SessionState>.value(sessionState),
      ),
      messageBadgeCountsProvider.overrideWith((Ref ref) async => messageBadges),
    ],
    child: child,
  );
}

class _EmptyForumRepository extends ForumRepository {
  _EmptyForumRepository() : super(ForumApi(Dio()));

  @override
  Future<List<Board>> boards() async => <Board>[];

  @override
  Future<List<Tag>> tags() async => <Tag>[];

  @override
  Future<ForumPageSlice<ThreadFeed>> threads({
    required ForumFeed feed,
    String? boardId,
    String? tag,
    String? cursor,
  }) async {
    return const ForumPageSlice<ThreadFeed>(
      items: <ThreadFeed>[],
      nextCursor: null,
      hasMore: false,
    );
  }
}

class _EmptyHomeRepository extends HomeRepository {
  _EmptyHomeRepository() : super(ActivityApi(Dio()), PlatformApi(Dio()));

  @override
  Future<List<Announcement>> announcements() async => <Announcement>[];

  @override
  Future<List<Promotion>> promotions() async => <Promotion>[];
}
