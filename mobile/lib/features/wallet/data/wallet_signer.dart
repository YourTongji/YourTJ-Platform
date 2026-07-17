import 'dart:convert';

import 'package:cryptography/cryptography.dart';

import 'wallet_seed_store.dart';

class LocalWalletKey {
  const LocalWalletKey({required this.publicKeyBase64});

  final String publicKeyBase64;
}

class WalletKeyUnavailable implements Exception {
  const WalletKeyUnavailable(this.message);

  final String message;

  @override
  String toString() => message;
}

class WalletSigner {
  WalletSigner(this._store, {Ed25519? algorithm})
    : _algorithm = algorithm ?? Ed25519();

  final WalletSeedStore _store;
  final Ed25519 _algorithm;

  Future<LocalWalletKey?> readPublicKey(String accountId) async {
    final List<int>? seed = await _readSeed(accountId);
    if (seed == null) {
      return null;
    }
    return _derivePublicKey(seed);
  }

  Future<LocalWalletKey> generate(String accountId) async {
    final SimpleKeyPair keyPair = await _algorithm.newKeyPair();
    final List<int> seed = await keyPair.extractPrivateKeyBytes();
    if (seed.length != 32) {
      throw const WalletKeyUnavailable('无法生成标准 Ed25519 钱包密钥');
    }
    final List<int> persistedSeed;
    try {
      persistedSeed = await _store.writeIfAbsent(accountId, seed);
    } on Object {
      throw const WalletKeyUnavailable('系统安全存储不可用，已停止创建钱包密钥');
    }
    return _derivePublicKey(persistedSeed);
  }

  Future<String> signExactBytes(
    String accountId,
    String expectedPublicKey,
    String signingBytes,
  ) async {
    final List<int>? seed = await _readSeed(accountId);
    if (seed == null) {
      throw const WalletKeyUnavailable('本机没有该账号的钱包密钥');
    }
    final SimpleKeyPair keyPair = await _algorithm.newKeyPairFromSeed(seed);
    final SimplePublicKey publicKey = await keyPair.extractPublicKey();
    if (base64Encode(publicKey.bytes) != expectedPublicKey) {
      throw const WalletKeyUnavailable('本机钱包私钥与服务端公钥不一致，已停止签名');
    }
    final Signature signature = await _algorithm.sign(
      utf8.encode(signingBytes),
      keyPair: keyPair,
    );
    return base64Encode(signature.bytes);
  }

  Future<void> delete(String accountId) async {
    try {
      await _store.delete(accountId);
    } on Object {
      throw const WalletKeyUnavailable('系统安全存储不可用，无法清除钱包密钥');
    }
  }

  Future<List<int>?> _readSeed(String accountId) async {
    try {
      return await _store.read(accountId);
    } on Object {
      throw const WalletKeyUnavailable('系统安全存储不可用，无法读取钱包密钥');
    }
  }

  Future<LocalWalletKey> _derivePublicKey(List<int> seed) async {
    final SimpleKeyPair keyPair = await _algorithm.newKeyPairFromSeed(seed);
    final SimplePublicKey publicKey = await keyPair.extractPublicKey();
    return LocalWalletKey(publicKeyBase64: base64Encode(publicKey.bytes));
  }
}
