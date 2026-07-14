import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';

final Provider<AnonymousAnnouncementSeenStore>
anonymousAnnouncementSeenStoreProvider =
    Provider<AnonymousAnnouncementSeenStore>(
      (Ref ref) => SharedPreferencesAnnouncementSeenStore(
        namespace: ref.watch(appServicesProvider).environment.storageNamespace,
      ),
    );

abstract interface class AnonymousAnnouncementSeenStore {
  Future<Set<String>> read();

  Future<void> remember(Announcement announcement);
}

class SharedPreferencesAnnouncementSeenStore
    implements AnonymousAnnouncementSeenStore {
  SharedPreferencesAnnouncementSeenStore({required String namespace})
    : _key =
          'yourtj.announcement.seenRevisions.v1.'
          '${Uri.encodeComponent(namespace)}';

  static const int _maximumEntries = 200;

  final String _key;

  @override
  Future<Set<String>> read() async {
    try {
      final SharedPreferences preferences =
          await SharedPreferences.getInstance();
      return (preferences.getStringList(_key) ?? <String>[]).toSet();
    } on Object {
      return <String>{};
    }
  }

  @override
  Future<void> remember(Announcement announcement) async {
    try {
      final SharedPreferences preferences =
          await SharedPreferences.getInstance();
      final List<String> entries =
          preferences.getStringList(_key) ?? <String>[];
      final String revisionKey = keyFor(announcement);
      entries.remove(revisionKey);
      entries.add(revisionKey);
      final int start = entries.length > _maximumEntries
          ? entries.length - _maximumEntries
          : 0;
      await preferences.setStringList(_key, entries.sublist(start));
    } on Object {
      // Anonymous seen state is a display enhancement, never an auth fact.
    }
  }

  static String keyFor(Announcement announcement) =>
      '${announcement.id}:${announcement.revision}';
}
