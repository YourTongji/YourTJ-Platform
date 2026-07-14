import 'dart:convert';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/auth/data/secure_session_storage.dart';

void main() {
  const String namespace = 'test_env';
  const String sessionKey = 'session.$namespace.v2.active-session';
  const String legacyAccountKey = 'session.$namespace.v1.active-account';
  const String legacyRefreshKey = 'session.$namespace.v1.refresh.account-a';

  test(
    'replaces the active credential with one secure-storage write',
    () async {
      final _FaultInjectingSecureStorage secrets =
          _FaultInjectingSecureStorage();
      final KeychainKeystoreSessionStorage storage =
          KeychainKeystoreSessionStorage(
            environmentNamespace: namespace,
            storage: secrets,
          );
      await storage.replaceSession(
        accountId: 'account-a',
        refreshToken: 'refresh-a',
      );
      secrets.writtenKeys.clear();

      await storage.replaceSession(
        accountId: 'account-b',
        refreshToken: 'refresh-b',
      );

      expect(secrets.writtenKeys, <String>[sessionKey]);
      expect(secrets.values.keys, <String>{sessionKey});
      expect(jsonDecode(secrets.values[sessionKey]!), <String, Object>{
        'schemaVersion': 2,
        'accountId': 'account-b',
        'refreshToken': 'refresh-b',
      });

      final StoredSessionCredential? afterRestart =
          await KeychainKeystoreSessionStorage(
            environmentNamespace: namespace,
            storage: secrets,
          ).readSession();
      expect(afterRestart?.accountId, 'account-b');
      expect(afterRestart?.refreshToken, 'refresh-b');
    },
  );

  test('failed replacement leaves the prior credential restartable', () async {
    final _FaultInjectingSecureStorage secrets = _FaultInjectingSecureStorage();
    final KeychainKeystoreSessionStorage storage =
        KeychainKeystoreSessionStorage(
          environmentNamespace: namespace,
          storage: secrets,
        );
    await storage.replaceSession(
      accountId: 'account-a',
      refreshToken: 'refresh-a',
    );
    secrets.failNextWrite(sessionKey);

    await expectLater(
      storage.replaceSession(accountId: 'account-b', refreshToken: 'refresh-b'),
      throwsStateError,
    );

    final StoredSessionCredential? afterRestart =
        await KeychainKeystoreSessionStorage(
          environmentNamespace: namespace,
          storage: secrets,
        ).readSession();
    expect(afterRestart?.accountId, 'account-a');
    expect(afterRestart?.refreshToken, 'refresh-a');
    expect(secrets.values.values, isNot(contains('refresh-b')));
  });

  test(
    'legacy cleanup failure cannot invalidate a committed migration',
    () async {
      final _FaultInjectingSecureStorage secrets = _FaultInjectingSecureStorage(
        <String, String>{
          legacyAccountKey: 'account-a',
          legacyRefreshKey: 'legacy-refresh',
        },
      )..failNextDelete(legacyRefreshKey);
      final KeychainKeystoreSessionStorage storage =
          KeychainKeystoreSessionStorage(
            environmentNamespace: namespace,
            storage: secrets,
          );

      final StoredSessionCredential? migrated = await storage.readSession();

      expect(migrated?.accountId, 'account-a');
      expect(migrated?.refreshToken, 'legacy-refresh');
      expect(secrets.values, contains(sessionKey));
      expect(secrets.values, contains(legacyAccountKey));

      final StoredSessionCredential? afterRestart =
          await KeychainKeystoreSessionStorage(
            environmentNamespace: namespace,
            storage: secrets,
          ).readSession();
      expect(afterRestart?.refreshToken, 'legacy-refresh');
      expect(secrets.values.keys, <String>{sessionKey});
    },
  );

  test('clear keeps the canonical record when legacy cleanup fails', () async {
    final _FaultInjectingSecureStorage secrets = _FaultInjectingSecureStorage();
    final KeychainKeystoreSessionStorage storage =
        KeychainKeystoreSessionStorage(
          environmentNamespace: namespace,
          storage: secrets,
        );
    await storage.replaceSession(
      accountId: 'account-a',
      refreshToken: 'refresh-a',
    );
    secrets.values[legacyAccountKey] = 'account-a';
    secrets.values[legacyRefreshKey] = 'legacy-refresh';
    secrets.failNextDelete(legacyRefreshKey);

    await expectLater(storage.clearSession('account-a'), throwsStateError);

    expect(secrets.values, contains(sessionKey));
    await storage.clearSession('account-a');
    expect(secrets.values, isEmpty);
  });

  test('stale account cleanup cannot delete another active account', () async {
    final _FaultInjectingSecureStorage secrets = _FaultInjectingSecureStorage();
    final KeychainKeystoreSessionStorage storage =
        KeychainKeystoreSessionStorage(
          environmentNamespace: namespace,
          storage: secrets,
        );
    await storage.replaceSession(
      accountId: 'account-b',
      refreshToken: 'refresh-b',
    );
    secrets.values[legacyAccountKey] = 'account-a';
    secrets.values[legacyRefreshKey] = 'legacy-refresh';

    await storage.clearSession('account-a');

    final StoredSessionCredential? active = await storage.readSession();
    expect(active?.accountId, 'account-b');
    expect(active?.refreshToken, 'refresh-b');
    expect(secrets.values.keys, <String>{sessionKey});
  });

  test('corrupt canonical record never falls back to a legacy token', () async {
    final _FaultInjectingSecureStorage secrets =
        _FaultInjectingSecureStorage(<String, String>{
          sessionKey: '{"schemaVersion":2,"accountId":"account-a"}',
          legacyAccountKey: 'account-a',
          legacyRefreshKey: 'legacy-refresh',
        });
    final KeychainKeystoreSessionStorage storage =
        KeychainKeystoreSessionStorage(
          environmentNamespace: namespace,
          storage: secrets,
        );

    await expectLater(storage.readSession(), throwsFormatException);

    expect(secrets.writtenKeys, isEmpty);
    expect(secrets.values[legacyRefreshKey], 'legacy-refresh');
  });
}

class _FaultInjectingSecureStorage extends FlutterSecureStorage {
  _FaultInjectingSecureStorage([Map<String, String>? initialValues])
    : values = <String, String>{...?initialValues};

  final Map<String, String> values;
  final List<String> writtenKeys = <String>[];
  final Set<String> _writeFailures = <String>{};
  final Set<String> _deleteFailures = <String>{};

  void failNextWrite(String key) => _writeFailures.add(key);

  void failNextDelete(String key) => _deleteFailures.add(key);

  @override
  Future<String?> read({
    required String key,
    AppleOptions? iOptions,
    AndroidOptions? aOptions,
    LinuxOptions? lOptions,
    WebOptions? webOptions,
    AppleOptions? mOptions,
    WindowsOptions? wOptions,
  }) async {
    return values[key];
  }

  @override
  Future<void> write({
    required String key,
    required String? value,
    AppleOptions? iOptions,
    AndroidOptions? aOptions,
    LinuxOptions? lOptions,
    WebOptions? webOptions,
    AppleOptions? mOptions,
    WindowsOptions? wOptions,
  }) async {
    writtenKeys.add(key);
    if (_writeFailures.remove(key)) {
      throw StateError('injected secure-storage write failure');
    }
    if (value == null) {
      values.remove(key);
    } else {
      values[key] = value;
    }
  }

  @override
  Future<void> delete({
    required String key,
    AppleOptions? iOptions,
    AndroidOptions? aOptions,
    LinuxOptions? lOptions,
    WebOptions? webOptions,
    AppleOptions? mOptions,
    WindowsOptions? wOptions,
  }) async {
    if (_deleteFailures.remove(key)) {
      throw StateError('injected secure-storage delete failure');
    }
    values.remove(key);
  }
}
