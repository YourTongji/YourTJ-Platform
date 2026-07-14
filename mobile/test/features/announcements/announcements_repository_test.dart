import 'package:dio/dio.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/announcements/data/announcements_repository.dart';
import 'package:yourtj_mobile/features/announcements/data/anonymous_announcement_seen_store.dart';

import '../auth/support/session_test_support.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('announcement receipt binds action to the displayed revision', () async {
    late final RecordingAdapter adapter;
    final Dio dio = Dio(BaseOptions(baseUrl: 'https://api.yourtj.de/api/v2'));
    adapter = RecordingAdapter((RequestOptions options) {
      expect(options.path, '/announcements/announcement-1/receipt');
      return jsonResponse(<String, Object?>{
        'revision': 3,
        'firstSeenAt': 100,
        'dismissedAt': null,
        'acknowledgedAt': 110,
      });
    });
    dio.httpClientAdapter = adapter;
    final AnnouncementsRepository repository = AnnouncementsRepository(
      PlatformApi(dio),
    );

    final AnnouncementReceipt result = await repository.record(
      announcement: _announcement(),
      action: AnnouncementReceiptInputActionEnum.acknowledge,
    );

    expect(result.revision, 3);
    expect(result.acknowledgedAt, 110);
    expect(requestJson(adapter.requests.single), <String, Object?>{
      'revision': 3,
      'action': 'acknowledge',
    });
  });

  test(
    'anonymous seen revisions are isolated by full environment namespace',
    () async {
      SharedPreferences.setMockInitialValues(<String, Object>{});
      final SharedPreferencesAnnouncementSeenStore production =
          SharedPreferencesAnnouncementSeenStore(namespace: 'production_full');
      final SharedPreferencesAnnouncementSeenStore preview =
          SharedPreferencesAnnouncementSeenStore(namespace: 'preview_full');

      await production.remember(_announcement());

      expect(
        await production.read(),
        contains(
          SharedPreferencesAnnouncementSeenStore.keyFor(_announcement()),
        ),
      );
      expect(await preview.read(), isEmpty);
    },
  );
}

Announcement _announcement() => Announcement(
  id: 'announcement-1',
  title: '重要公告',
  status: AnnouncementStatusEnum.published,
  effectiveState: AnnouncementEffectiveStateEnum.active,
  presentation: AnnouncementPresentationEnum.card,
  severity: AnnouncementSeverityEnum.warning,
  priority: 10,
  audience: AnnouncementAudienceEnum.all,
  requiresAck: true,
  version: 4,
  revision: 3,
  createdAt: 90,
  updatedAt: 100,
);
