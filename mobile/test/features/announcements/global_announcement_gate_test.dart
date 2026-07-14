import 'package:dio/dio.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/app/app_services.dart';
import 'package:yourtj_mobile/features/announcements/data/announcements_repository.dart';
import 'package:yourtj_mobile/features/announcements/data/anonymous_announcement_seen_store.dart';
import 'package:yourtj_mobile/features/announcements/presentation/global_announcement_gate.dart';
import 'package:yourtj_mobile/features/auth/domain/session_state.dart';

void main() {
  testWidgets(
    'shows the authenticated unread queue and records exact actions',
    (WidgetTester tester) async {
      final GlobalKey<NavigatorState> navigatorKey =
          GlobalKey<NavigatorState>();
      final _RecordingAnnouncementsRepository repository =
          _RecordingAnnouncementsRepository();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            announcementsRepositoryProvider.overrideWithValue(repository),
            anonymousAnnouncementSeenStoreProvider.overrideWithValue(
              _MemoryAnnouncementSeenStore(),
            ),
            sessionStateProvider.overrideWith(
              (Ref ref) => Stream<SessionState>.value(
                SessionState.authenticated(generation: 1, account: _account()),
              ),
            ),
          ],
          child: MaterialApp(
            navigatorKey: navigatorKey,
            home: GlobalAnnouncementGate(
              navigatorKey: navigatorKey,
              child: const Scaffold(body: Text('首页内容')),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('全局公告'), findsOneWidget);
      expect(find.text('首页内容'), findsOneWidget);

      await tester.tap(find.text('知道了'));
      await tester.pumpAndSettle();

      expect(repository.actions, <AnnouncementReceiptInputActionEnum>[
        AnnouncementReceiptInputActionEnum.seen,
        AnnouncementReceiptInputActionEnum.dismiss,
      ]);
      expect(find.text('全局公告'), findsNothing);
    },
  );

  testWidgets('anonymous revision is remembered without server receipts', (
    WidgetTester tester,
  ) async {
    final _RecordingAnnouncementsRepository repository =
        _RecordingAnnouncementsRepository();
    final _MemoryAnnouncementSeenStore seenStore =
        _MemoryAnnouncementSeenStore();

    await _pumpGate(
      tester,
      viewer: const SessionState.anonymous(generation: 1),
      repository: repository,
      seenStore: seenStore,
    );

    expect(find.text('全局公告'), findsOneWidget);
    await tester.tap(find.text('知道了'));
    await tester.pumpAndSettle();

    expect(repository.actions, isEmpty);
    expect(seenStore.values, <String>{
      SharedPreferencesAnnouncementSeenStore.keyFor(_announcement()),
    });

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    await _pumpGate(
      tester,
      viewer: const SessionState.anonymous(generation: 2),
      repository: repository,
      seenStore: seenStore,
    );

    expect(find.text('全局公告'), findsNothing);
  });

  testWidgets('required acknowledgement stays single on app resume', (
    WidgetTester tester,
  ) async {
    final _RecordingAnnouncementsRepository repository =
        _RecordingAnnouncementsRepository(
          unread: <Announcement>[_announcement(requiresAck: true)],
        );

    await _pumpGate(
      tester,
      viewer: SessionState.authenticated(generation: 1, account: _account()),
      repository: repository,
      seenStore: _MemoryAnnouncementSeenStore(),
    );

    expect(find.text('全局公告'), findsOneWidget);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(find.text('全局公告'), findsOneWidget);

    await tester.tapAt(const Offset(4, 4));
    await tester.pump();
    expect(find.text('全局公告'), findsOneWidget);

    await tester.tap(find.text('我已知晓'));
    await tester.pumpAndSettle();
    expect(repository.actions, <AnnouncementReceiptInputActionEnum>[
      AnnouncementReceiptInputActionEnum.seen,
      AnnouncementReceiptInputActionEnum.acknowledge,
    ]);
  });
}

Future<void> _pumpGate(
  WidgetTester tester, {
  required SessionState viewer,
  required _RecordingAnnouncementsRepository repository,
  required _MemoryAnnouncementSeenStore seenStore,
}) async {
  final GlobalKey<NavigatorState> navigatorKey = GlobalKey<NavigatorState>();
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        announcementsRepositoryProvider.overrideWithValue(repository),
        anonymousAnnouncementSeenStoreProvider.overrideWithValue(seenStore),
        sessionStateProvider.overrideWith(
          (Ref ref) => Stream<SessionState>.value(viewer),
        ),
      ],
      child: MaterialApp(
        navigatorKey: navigatorKey,
        home: GlobalAnnouncementGate(
          navigatorKey: navigatorKey,
          child: const Scaffold(body: Text('首页内容')),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
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

Announcement _announcement({bool requiresAck = false}) => Announcement(
  id: 'announcement-1',
  title: '全局公告',
  body: '这条公告会在任意页面展示。',
  status: AnnouncementStatusEnum.published,
  effectiveState: AnnouncementEffectiveStateEnum.active,
  presentation: AnnouncementPresentationEnum.card,
  severity: AnnouncementSeverityEnum.info,
  priority: 10,
  audience: AnnouncementAudienceEnum.all,
  requiresAck: requiresAck,
  version: 1,
  revision: 2,
  createdAt: 1,
  updatedAt: 2,
);

class _RecordingAnnouncementsRepository extends AnnouncementsRepository {
  _RecordingAnnouncementsRepository({List<Announcement>? unread})
    : unreadItems = unread ?? <Announcement>[_announcement()],
      super(PlatformApi(Dio()));

  final List<AnnouncementReceiptInputActionEnum> actions =
      <AnnouncementReceiptInputActionEnum>[];
  final List<Announcement> unreadItems;

  @override
  Future<List<Announcement>> active() async => unreadItems;

  @override
  Future<List<Announcement>> unread() async => unreadItems;

  @override
  Future<AnnouncementReceipt> record({
    required Announcement announcement,
    required AnnouncementReceiptInputActionEnum action,
  }) async {
    actions.add(action);
    return AnnouncementReceipt(revision: announcement.revision);
  }
}

class _MemoryAnnouncementSeenStore implements AnonymousAnnouncementSeenStore {
  final Set<String> values = <String>{};

  @override
  Future<Set<String>> read() async => Set<String>.from(values);

  @override
  Future<void> remember(Announcement announcement) async {
    values.add(SharedPreferencesAnnouncementSeenStore.keyFor(announcement));
  }
}
