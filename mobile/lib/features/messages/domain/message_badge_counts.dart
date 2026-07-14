import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../../auth/domain/session_state.dart';
import '../../notifications/data/notifications_repository.dart';
import '../data/messages_repository.dart';

class MessageBadgeCounts {
  const MessageBadgeCounts({
    required this.notifications,
    required this.governance,
    required this.directMessages,
    required this.requests,
  });

  static const MessageBadgeCounts zero = MessageBadgeCounts(
    notifications: 0,
    governance: 0,
    directMessages: 0,
    requests: 0,
  );

  final int notifications;
  final int governance;
  final int directMessages;
  final int requests;

  int get total => notifications + governance + directMessages + requests;
}

final FutureProvider<MessageBadgeCounts> messageBadgeCountsProvider =
    FutureProvider<MessageBadgeCounts>((Ref ref) async {
      final SessionState viewer = await ref.watch(sessionStateProvider.future);
      if (!viewer.isAuthenticated) {
        return MessageBadgeCounts.zero;
      }
      final NotificationsRepository notifications = ref.watch(
        notificationsRepositoryProvider,
      );
      final MessagesRepository messages = ref.watch(messagesRepositoryProvider);
      final Future<DmCounts> directMessages = _dmCountsOrZero(
        messages.counts(),
      );
      final List<int> counts = await Future.wait<int>(<Future<int>>[
        _countOrZero(notifications.unreadCount()),
        _countOrZero(notifications.governanceUnreadCount()),
      ]);
      final DmCounts dmCounts = await directMessages;
      return MessageBadgeCounts(
        notifications: counts[0],
        governance: counts[1],
        directMessages: dmCounts.unreadCount,
        requests: dmCounts.requestCount,
      );
    });

Future<int> _countOrZero(Future<int> request) async {
  try {
    return await request;
  } on Object {
    return 0;
  }
}

Future<DmCounts> _dmCountsOrZero(Future<DmCounts> request) async {
  try {
    return await request;
  } on Object {
    return DmCounts(count: 0, unreadCount: 0, requestCount: 0);
  }
}
