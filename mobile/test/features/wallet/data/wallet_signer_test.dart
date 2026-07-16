import 'dart:convert';
import 'dart:io';

import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_seed_store.dart';
import 'package:yourtj_mobile/features/wallet/data/wallet_signer.dart';

void main() {
  group('WalletSigner cross-client contract', () {
    late _MemoryWalletSeedStore store;
    late WalletSigner signer;

    setUp(() {
      store = _MemoryWalletSeedStore();
      signer = WalletSigner(store);
    });

    test('matches every shared Ed25519 exact-byte vector', () async {
      final Map<String, dynamic> fixture =
          jsonDecode(
                File(
                  '../contract/fixtures/wallet-signing-v1.json',
                ).readAsStringSync(),
              )
              as Map<String, dynamic>;
      final List<dynamic> vectors = fixture['vectors']! as List<dynamic>;

      for (final dynamic rawVector in vectors) {
        final Map<String, dynamic> vector = rawVector as Map<String, dynamic>;
        final String vectorId = vector['id']! as String;
        store.replaceForTest(
          'fixture-account',
          _decodeHex(vector['seedHex']! as String),
        );

        final LocalWalletKey? key = await signer.readPublicKey(
          'fixture-account',
        );
        final String signature = await signer.signExactBytes(
          'fixture-account',
          vector['publicKeyBase64']! as String,
          vector['signingBytes']! as String,
        );

        expect(
          key?.publicKeyBase64,
          vector['publicKeyBase64'],
          reason: vectorId,
        );
        expect(signature, vector['signatureBase64'], reason: vectorId);
      }
    });

    test('keeps seeds account-scoped and deletion is fail-closed', () async {
      await signer.generate('account-one');

      expect(await signer.readPublicKey('account-one'), isNotNull);
      expect(await signer.readPublicKey('account-two'), isNull);
      expect(
        () =>
            signer.signExactBytes('account-two', 'unused-public-key', 'intent'),
        throwsA(isA<WalletKeyUnavailable>()),
      );

      await signer.delete('account-one');

      expect(await signer.readPublicKey('account-one'), isNull);
      expect(
        () =>
            signer.signExactBytes('account-one', 'unused-public-key', 'intent'),
        throwsA(isA<WalletKeyUnavailable>()),
      );
    });

    test(
      'concurrent generation returns the one persisted account key',
      () async {
        final List<LocalWalletKey> generated = await Future.wait(
          <Future<LocalWalletKey>>[
            signer.generate('account-one'),
            signer.generate('account-one'),
          ],
        );

        expect(
          generated.map((LocalWalletKey key) => key.publicKeyBase64).toSet(),
          hasLength(1),
        );
        expect(store.persistedWrites, 1);
        expect(
          (await signer.readPublicKey('account-one'))?.publicKeyBase64,
          generated.first.publicKeyBase64,
        );
      },
    );

    test(
      'refuses to sign when the actual seed no longer matches the expected key',
      () async {
        final LocalWalletKey original = await signer.generate('account-one');
        store.replaceForTest('account-one', List<int>.filled(32, 0x5a));

        await expectLater(
          signer.signExactBytes(
            'account-one',
            original.publicKeyBase64,
            'intent',
          ),
          throwsA(isA<WalletKeyUnavailable>()),
        );
      },
    );

    test('maps secure-storage failures to a wallet boundary error', () async {
      final WalletSigner unavailableSigner = WalletSigner(
        _UnavailableWalletSeedStore(),
      );

      await expectLater(
        unavailableSigner.readPublicKey('account-one'),
        throwsA(isA<WalletKeyUnavailable>()),
      );
      await expectLater(
        unavailableSigner.generate('account-one'),
        throwsA(isA<WalletKeyUnavailable>()),
      );
      await expectLater(
        unavailableSigner.delete('account-one'),
        throwsA(isA<WalletKeyUnavailable>()),
      );
    });
  });

  test('secure seed store keeps the first concurrent account seed', () async {
    FlutterSecureStorage.setMockInitialValues(<String, String>{});
    const FlutterSecureStorage secureStorage = FlutterSecureStorage();
    final KeychainKeystoreWalletSeedStore store =
        KeychainKeystoreWalletSeedStore(
          environmentNamespace: 'environment_a',
          storage: secureStorage,
        );
    final List<int> firstSeed = List<int>.filled(32, 0x11);
    final List<int> secondSeed = List<int>.filled(32, 0x22);

    final List<List<int>> persisted = await Future.wait(<Future<List<int>>>[
      store.writeIfAbsent('account-one', firstSeed),
      store.writeIfAbsent('account-one', secondSeed),
    ]);

    expect(persisted, everyElement(firstSeed));
    expect(await store.read('account-one'), firstSeed);
  });
}

List<int> _decodeHex(String value) {
  return <int>[
    for (int offset = 0; offset < value.length; offset += 2)
      int.parse(value.substring(offset, offset + 2), radix: 16),
  ];
}

class _MemoryWalletSeedStore implements WalletSeedStore {
  final Map<String, List<int>> _seeds = <String, List<int>>{};
  int persistedWrites = 0;

  void replaceForTest(String accountId, List<int> seed) {
    _seeds[accountId] = List<int>.from(seed);
  }

  @override
  Future<void> delete(String accountId) async {
    _seeds.remove(accountId);
  }

  @override
  Future<List<int>?> read(String accountId) async {
    final List<int>? seed = _seeds[accountId];
    return seed == null ? null : List<int>.from(seed);
  }

  @override
  Future<List<int>> writeIfAbsent(String accountId, List<int> seed) async {
    final List<int>? existing = _seeds[accountId];
    if (existing != null) {
      return List<int>.from(existing);
    }
    final List<int> persisted = List<int>.from(seed);
    _seeds[accountId] = persisted;
    persistedWrites += 1;
    return List<int>.from(persisted);
  }
}

class _UnavailableWalletSeedStore implements WalletSeedStore {
  @override
  Future<void> delete(String accountId) => throw StateError('unavailable');

  @override
  Future<List<int>?> read(String accountId) => throw StateError('unavailable');

  @override
  Future<List<int>> writeIfAbsent(String accountId, List<int> seed) =>
      throw StateError('unavailable');
}
