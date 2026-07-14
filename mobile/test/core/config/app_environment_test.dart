import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/config/app_environment.dart';

void main() {
  group('AppEnvironment validation', () {
    test('accepts the canonical HTTPS API base', () {
      final AppEnvironment environment = AppEnvironment(
        apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
      );

      expect(environment.apiBaseUri, Uri.parse('https://api.yourtj.de/api/v2'));
      expect(environment.mediaCdnBaseUri, Uri.parse('https://api.yourtj.de'));
      expect(environment.storageNamespace, isNotEmpty);
    });

    test('isolates secure storage by the complete API environment', () {
      final AppEnvironment production = AppEnvironment(
        apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
      );
      final AppEnvironment staging = AppEnvironment(
        apiBaseUri: Uri.parse('https://pf-dev.yourtj.de/api/v2'),
      );

      expect(production.storageNamespace, isNot(staging.storageNamespace));
      expect(
        AppEnvironment(
          apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
        ).storageNamespace,
        production.storageNamespace,
      );
      expect(production.storageNamespace, matches(RegExp(r'^[A-Za-z0-9_-]+$')));
    });

    test('accepts loopback HTTP only in a debug build', () {
      expect(
        AppEnvironment(
          apiBaseUri: Uri.parse('http://127.0.0.1:3000/api/v2'),
        ).apiBaseUri,
        Uri.parse('http://127.0.0.1:3000/api/v2'),
      );
      expect(
        AppEnvironment(
          apiBaseUri: Uri.parse('http://[::1]:3000/api/v2'),
        ).apiBaseUri,
        Uri.parse('http://[::1]:3000/api/v2'),
      );
    });

    test('rejects plaintext non-loopback origins', () {
      expect(
        () => AppEnvironment(
          apiBaseUri: Uri.parse('http://api.yourtj.de/api/v2'),
        ),
        throwsFormatException,
      );
    });

    test('validates an explicit media CDN origin', () {
      final AppEnvironment environment = AppEnvironment(
        apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
        mediaCdnBaseUri: Uri.parse('https://media.yourtj.de'),
      );

      expect(
        environment.ownsPlatformMedia(
          Uri.parse(
            'https://media.yourtj.de/assets/avatar.webp?auth_key=short',
          ),
        ),
        isTrue,
      );
      expect(
        environment.ownsPlatformMedia(
          Uri.parse('https://api.yourtj.de/api/v2/media/avatar'),
        ),
        isTrue,
      );
      expect(
        environment.ownsPlatformMedia(
          Uri.parse('https://media.yourtj.de.tracker.example/avatar.webp'),
        ),
        isFalse,
      );
    });

    test('rejects a media CDN value that is not an HTTPS origin', () {
      for (final String candidate in <String>[
        'http://media.yourtj.de',
        'https://user@media.yourtj.de',
        'https://media.yourtj.de/assets',
        'https://media.yourtj.de?token=secret',
      ]) {
        expect(
          () => AppEnvironment(
            apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
            mediaCdnBaseUri: Uri.parse(candidate),
          ),
          throwsFormatException,
          reason: candidate,
        );
      }
    });

    test('rejects missing API path, query, and fragment', () {
      for (final String candidate in <String>[
        'https://api.yourtj.de/',
        'https://api.yourtj.de/api/v1',
        'https://api.yourtj.de/api/v2?token=secret',
        'https://api.yourtj.de/api/v2#fragment',
      ]) {
        expect(
          () => AppEnvironment(apiBaseUri: Uri.parse(candidate)),
          throwsFormatException,
          reason: candidate,
        );
      }
    });
  });

  group('AppEnvironment origin ownership', () {
    final AppEnvironment environment = AppEnvironment(
      apiBaseUri: Uri.parse('https://api.yourtj.de/api/v2'),
    );

    test('owns only the configured origin and API path boundary', () {
      expect(
        environment.owns(Uri.parse('https://api.yourtj.de/api/v2')),
        isTrue,
      );
      expect(
        environment.owns(
          Uri.parse('https://api.yourtj.de/api/v2/forum/threads?q=tea'),
        ),
        isTrue,
      );
      expect(
        environment.owns(Uri.parse('https://api.yourtj.de/api/v20')),
        isFalse,
      );
      expect(
        environment.owns(Uri.parse('https://api.yourtj.de/other/api/v2')),
        isFalse,
      );
    });

    test('rejects sibling hosts, ports, and schemes', () {
      for (final String candidate in <String>[
        'https://evil.example/api/v2',
        'https://api.yourtj.de.evil.example/api/v2',
        'https://api.yourtj.de:444/api/v2',
        'http://api.yourtj.de/api/v2',
      ]) {
        expect(
          environment.owns(Uri.parse(candidate)),
          isFalse,
          reason: candidate,
        );
      }
    });
  });
}
