import 'package:dio/dio.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';
import 'package:yourtj_mobile/features/messages/data/messages_repository.dart';
import 'package:yourtj_mobile/features/messages/domain/message_badge_counts.dart';
import 'package:yourtj_mobile/features/notifications/data/notifications_repository.dart';

void main() {
  test('aggregates notification, governance, DM, and request counts', () async {
    final _BadgeNotificationsRepository notifications =
        _BadgeNotificationsRepository(notificationCount: 2, governanceCount: 3);
    final _BadgeMessagesRepository messages = _BadgeMessagesRepository(
      dmCounts: DmCounts(count: 9, unreadCount: 4, requestCount: 5),
    );
    final ProviderContainer container = _container(
      viewer: SessionState.authenticated(generation: 1, account: _account()),
      notifications: notifications,
      messages: messages,
    );
    addTearDown(container.dispose);

    final MessageBadgeCounts counts = await container.read(
      messageBadgeCountsProvider.future,
    );

    expect(counts.notifications, 2);
    expect(counts.governance, 3);
    expect(counts.directMessages, 4);
    expect(counts.requests, 5);
    expect(counts.total, 14);
  });

  test('anonymous viewer does not request private badge endpoints', () async {
    final _BadgeNotificationsRepository notifications =
        _BadgeNotificationsRepository(notificationCount: 2, governanceCount: 3);
    final _BadgeMessagesRepository messages = _BadgeMessagesRepository(
      dmCounts: DmCounts(count: 9, unreadCount: 4, requestCount: 5),
    );
    final ProviderContainer container = _container(
      viewer: const SessionState.anonymous(generation: 1),
      notifications: notifications,
      messages: messages,
    );
    addTearDown(container.dispose);

    final MessageBadgeCounts counts = await container.read(
      messageBadgeCountsProvider.future,
    );

    expect(counts.total, 0);
    expect(notifications.requestCount, 0);
    expect(messages.requestCount, 0);
  });

  test(
    'one unavailable source does not hide counts from healthy sources',
    () async {
      final _BadgeNotificationsRepository notifications =
          _BadgeNotificationsRepository(
            notificationCount: 2,
            governanceCount: 3,
            failGovernance: true,
          );
      final _BadgeMessagesRepository messages = _BadgeMessagesRepository(
        dmCounts: DmCounts(count: 9, unreadCount: 4, requestCount: 5),
      );
      final ProviderContainer container = _container(
        viewer: SessionState.authenticated(generation: 1, account: _account()),
        notifications: notifications,
        messages: messages,
      );
      addTearDown(container.dispose);

      final MessageBadgeCounts counts = await container.read(
        messageBadgeCountsProvider.future,
      );

      expect(counts.notifications, 2);
      expect(counts.governance, 0);
      expect(counts.directMessages, 4);
      expect(counts.requests, 5);
    },
  );
}

ProviderContainer _container({
  required SessionState viewer,
  required NotificationsRepository notifications,
  required MessagesRepository messages,
}) {
  return ProviderContainer(
    overrides: [
      sessionStateProvider.overrideWithValue(
        AsyncValue<SessionState>.data(viewer),
      ),
      notificationsRepositoryProvider.overrideWithValue(notifications),
      messagesRepositoryProvider.overrideWithValue(messages),
    ],
  );
}

Account _account() => Account(
  id: 'account-1',
  handle: 'student',
  avatarUrl: null,
  role: AccountRoleEnum.user,
  capabilities: const <String>[],
  trustLevel: 1,
  hasPassword: true,
  onboardingRequired: false,
  createdAt: 1,
);

class _BadgeNotificationsRepository extends NotificationsRepository {
  _BadgeNotificationsRepository({
    required this.notificationCount,
    required this.governanceCount,
    this.failGovernance = false,
  }) : super(NotificationsApi(Dio()));

  final int notificationCount;
  final int governanceCount;
  final bool failGovernance;
  int requestCount = 0;

  @override
  Future<int> unreadCount() async {
    requestCount += 1;
    return notificationCount;
  }

  @override
  Future<int> governanceUnreadCount() async {
    requestCount += 1;
    if (failGovernance) {
      throw StateError('governance unavailable');
    }
    return governanceCount;
  }
}

class _BadgeMessagesRepository extends MessagesRepository {
  _BadgeMessagesRepository({required this.dmCounts}) : super(ForumApi(Dio()));

  final DmCounts dmCounts;
  int requestCount = 0;

  @override
  Future<DmCounts> counts() async {
    requestCount += 1;
    return dmCounts;
  }
}
