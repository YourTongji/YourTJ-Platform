import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/core/storage/installation_store.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('SharedPreferencesInstallationStore', () {
    test(
      'isolates installation identifiers by environment namespace',
      () async {
        final _MemoryInstallationPreferences preferences =
            _MemoryInstallationPreferences();
        final SharedPreferencesInstallationStore production =
            SharedPreferencesInstallationStore(
              environmentNamespace: 'production_namespace',
              preferences: preferences,
            );
        final SharedPreferencesInstallationStore preview =
            SharedPreferencesInstallationStore(
              environmentNamespace: 'preview_namespace',
              preferences: preferences,
            );

        final String productionId = await production.readOrCreateId();
        final String previewId = await preview.readOrCreateId();

        expect(productionId, isNot(previewId));
        expect(await production.readOrCreateId(), productionId);
        expect(await preview.readOrCreateId(), previewId);
        expect(
          preferences.values,
          containsPair(
            'yourtj.installation.v1.production_namespace',
            productionId,
          ),
        );
        expect(
          preferences.values,
          containsPair('yourtj.installation.v1.preview_namespace', previewId),
        );
      },
    );

    test('replaces a malformed persisted identifier with a UUID v4', () async {
      final _MemoryInstallationPreferences preferences =
          _MemoryInstallationPreferences()
            ..values['yourtj.installation.v1.production'] = 'not-a-uuid';
      final SharedPreferencesInstallationStore store =
          SharedPreferencesInstallationStore(
            environmentNamespace: 'production',
            preferences: preferences,
            uuidV4: () => _validUuid,
          );

      expect(await store.readOrCreateId(), _validUuid);
      expect(
        preferences.values['yourtj.installation.v1.production'],
        _validUuid,
      );
    });

    test('does not persist a generated value that is not UUID v4', () async {
      final _MemoryInstallationPreferences preferences =
          _MemoryInstallationPreferences();
      final SharedPreferencesInstallationStore store =
          SharedPreferencesInstallationStore(
            environmentNamespace: 'production',
            preferences: preferences,
            uuidV4: () => '01944f75-f64a-75f4-8f58-c840b42334a6',
          );

      await expectLater(
        store.readOrCreateId(),
        throwsA(isA<InstallationStoreException>()),
      );
      expect(preferences.values, isEmpty);
    });
  });

  group('IosNoBackupInstallationStore', () {
    test('passes the environment namespace to the native boundary', () async {
      final _FakeIosInstallationIdChannel channel =
          _FakeIosInstallationIdChannel(result: _validUuid.toUpperCase());
      final IosNoBackupInstallationStore store = IosNoBackupInstallationStore(
        environmentNamespace: 'preview_namespace',
        channel: channel,
      );

      expect(await store.readOrCreateId(), _validUuid);
      expect(channel.namespaces, <String>['preview_namespace']);
    });

    test(
      'fails closed when native storage returns a malformed value',
      () async {
        final IosNoBackupInstallationStore store = IosNoBackupInstallationStore(
          environmentNamespace: 'production',
          channel: _FakeIosInstallationIdChannel(result: 'not-a-uuid'),
        );

        await expectLater(
          store.readOrCreateId(),
          throwsA(isA<InstallationStoreException>()),
        );
      },
    );

    test('propagates a native boundary failure without fallback', () async {
      final _MemoryInstallationPreferences preferences =
          _MemoryInstallationPreferences();
      final InstallationStore store = createInstallationStore(
        environmentNamespace: 'production',
        isWeb: false,
        targetPlatform: TargetPlatform.iOS,
        preferences: preferences,
        iosChannel: _ThrowingIosInstallationIdChannel(),
      );

      await expectLater(store.readOrCreateId(), throwsStateError);
      expect(preferences.values, isEmpty);
    });
  });

  group('MethodChannelIosInstallationIdChannel', () {
    const MethodChannel channel = MethodChannel(
      'de.yourtj.mobile/installation.test',
    );

    tearDown(() async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, null);
    });

    test('maps platform failures to a closed storage failure', () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            throw PlatformException(code: 'STORE_UNAVAILABLE');
          });
      final MethodChannelIosInstallationIdChannel nativeChannel =
          MethodChannelIosInstallationIdChannel(channel: channel);

      await expectLater(
        nativeChannel.readOrCreateId('production'),
        throwsA(isA<InstallationStoreException>()),
      );
    });

    test('uses only the narrow read-or-create method', () async {
      MethodCall? receivedCall;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            receivedCall = call;
            return _validUuid;
          });
      final MethodChannelIosInstallationIdChannel nativeChannel =
          MethodChannelIosInstallationIdChannel(channel: channel);

      expect(await nativeChannel.readOrCreateId('preview'), _validUuid);
      expect(receivedCall?.method, 'readOrCreateInstallationId');
      expect(receivedCall?.arguments, <String, String>{
        'environmentNamespace': 'preview',
      });
    });
  });

  group('createInstallationStore', () {
    test('uses native no-backup storage only for non-web iOS', () {
      final _MemoryInstallationPreferences preferences =
          _MemoryInstallationPreferences();
      expect(
        createInstallationStore(
          environmentNamespace: 'production',
          isWeb: false,
          targetPlatform: TargetPlatform.iOS,
        ),
        isA<IosNoBackupInstallationStore>(),
      );
      expect(
        createInstallationStore(
          environmentNamespace: 'production',
          isWeb: false,
          targetPlatform: TargetPlatform.android,
          preferences: preferences,
        ),
        isA<SharedPreferencesInstallationStore>(),
      );
      expect(
        createInstallationStore(
          environmentNamespace: 'production',
          isWeb: true,
          targetPlatform: TargetPlatform.iOS,
          preferences: preferences,
        ),
        isA<SharedPreferencesInstallationStore>(),
      );
    });
  });

  test('rejects unsafe environment namespaces on every storage path', () {
    for (final String namespace in <String>[
      '',
      'https://api.yourtj.de/api/v2',
      'production\npreview',
      'a' * 201,
    ]) {
      expect(
        () =>
            SharedPreferencesInstallationStore(environmentNamespace: namespace),
        throwsArgumentError,
      );
      expect(
        () => IosNoBackupInstallationStore(environmentNamespace: namespace),
        throwsArgumentError,
      );
    }
  });
}

const String _validUuid = '9b636a88-7d22-4c2f-9ad0-0dd9b80735f1';

class _MemoryInstallationPreferences implements InstallationPreferences {
  final Map<String, String> values = <String, String>{};

  @override
  Future<String?> getString(String key) async => values[key];

  @override
  Future<void> setString(String key, String value) async {
    values[key] = value;
  }
}

class _FakeIosInstallationIdChannel implements IosInstallationIdChannel {
  _FakeIosInstallationIdChannel({required this.result});

  final String result;
  final List<String> namespaces = <String>[];

  @override
  Future<String> readOrCreateId(String environmentNamespace) async {
    namespaces.add(environmentNamespace);
    return result;
  }
}

class _ThrowingIosInstallationIdChannel implements IosInstallationIdChannel {
  @override
  Future<String> readOrCreateId(String environmentNamespace) async {
    throw StateError('native store unavailable');
  }
}
