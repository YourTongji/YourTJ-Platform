import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/navigation/external_link.dart';

void main() {
  test('external links accept only credential-free absolute HTTPS URLs', () {
    expect(
      isAllowedExternalHttps(Uri.parse('https://example.edu/path?q=course')),
      isTrue,
    );

    for (final String candidate in <String>[
      'http://example.edu/path',
      'javascript:alert(1)',
      'https:relative-path',
      'https://user:password@example.edu/path',
      '/internal/path',
    ]) {
      expect(
        isAllowedExternalHttps(Uri.parse(candidate)),
        isFalse,
        reason: candidate,
      );
    }
  });
}
