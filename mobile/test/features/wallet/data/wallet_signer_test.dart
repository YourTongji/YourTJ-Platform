import 'dart:convert';
import 'dart:io';

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
        await store.write(
          'fixture-account',
          _decodeHex(vector['seedHex']! as String),
        );

        final LocalWalletKey? key = await signer.readPublicKey(
          'fixture-account',
        );
        final String signature = await signer.signExactBytes(
          'fixture-account',
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
        () => signer.signExactBytes('account-two', 'intent'),
        throwsA(isA<WalletKeyUnavailable>()),
      );

      await signer.delete('account-one');

      expect(await signer.readPublicKey('account-one'), isNull);
      expect(
        () => signer.signExactBytes('account-one', 'intent'),
        throwsA(isA<WalletKeyUnavailable>()),
      );
    });

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
}

List<int> _decodeHex(String value) {
  return <int>[
    for (int offset = 0; offset < value.length; offset += 2)
      int.parse(value.substring(offset, offset + 2), radix: 16),
  ];
}

class _MemoryWalletSeedStore implements WalletSeedStore {
  final Map<String, List<int>> _seeds = <String, List<int>>{};

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
  Future<void> write(String accountId, List<int> seed) async {
    _seeds[accountId] = List<int>.from(seed);
  }
}

class _UnavailableWalletSeedStore implements WalletSeedStore {
  @override
  Future<void> delete(String accountId) => throw StateError('unavailable');

  @override
  Future<List<int>?> read(String accountId) => throw StateError('unavailable');

  @override
  Future<void> write(String accountId, List<int> seed) =>
      throw StateError('unavailable');
}
