import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

const String accountExportFileName = 'yourtj-account-export.json';
const int accountExportMaxPayloadBytes = 16 * 1024 * 1024;

final Provider<AccountExportFileSaver> accountExportFileSaverProvider =
    Provider<AccountExportFileSaver>(
      (Ref ref) => MethodChannelAccountExportFileSaver(),
    );

enum AccountExportSaveResult { saved, cancelled }

enum AccountExportSaveFailureKind {
  invalidPayload,
  payloadTooLarge,
  busy,
  unavailable,
  writeFailed,
  cleanupFailed,
  sessionChanged,
  unexpected,
}

class AccountExportSaveFailure implements Exception {
  const AccountExportSaveFailure({required this.kind, required this.message});

  final AccountExportSaveFailureKind kind;
  final String message;

  @override
  String toString() => message;
}

abstract interface class AccountExportFileSaver {
  Future<AccountExportSaveResult> save(AccountDataExport export);

  Future<void> cancelPendingSave();
}

class MethodChannelAccountExportFileSaver implements AccountExportFileSaver {
  MethodChannelAccountExportFileSaver({
    MethodChannel? channel,
    int maxPayloadBytes = accountExportMaxPayloadBytes,
  }) : _channel = channel ?? const MethodChannel(_channelName),
       _maxPayloadBytes = maxPayloadBytes {
    if (maxPayloadBytes <= 0 ||
        maxPayloadBytes > accountExportMaxPayloadBytes) {
      throw ArgumentError.value(
        maxPayloadBytes,
        'maxPayloadBytes',
        'must be between 1 and $accountExportMaxPayloadBytes',
      );
    }
  }

  static const String _channelName = 'de.yourtj.mobile/account-export';
  static const String _saveMethod = 'saveAccountExport';
  static const String _cancelMethod = 'cancelAccountExport';
  static const String _expectedSchemaVersion = 'yourtj.account-export.v2';
  static const Set<String> _expectedSections = <String>{
    'identity',
    'forum',
    'reviews',
    'governance',
    'credit',
    'activity',
    'platform',
    'mediaMetadata',
  };

  final MethodChannel _channel;
  final int _maxPayloadBytes;

  @override
  Future<AccountExportSaveResult> save(AccountDataExport export) async {
    final Uint8List bytes = _encode(export);
    try {
      final String? result = await _channel.invokeMethod<String>(
        _saveMethod,
        <String, Object>{'fileName': accountExportFileName, 'bytes': bytes},
      );
      return switch (result) {
        'saved' => AccountExportSaveResult.saved,
        'cancelled' => AccountExportSaveResult.cancelled,
        _ => throw const AccountExportSaveFailure(
          kind: AccountExportSaveFailureKind.unexpected,
          message: '系统没有确认导出文件的保存结果，未显示成功状态。',
        ),
      };
    } on AccountExportSaveFailure {
      rethrow;
    } on PlatformException catch (exception) {
      throw _failureForPlatformCode(exception.code);
    } on MissingPluginException {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.unavailable,
        message: '当前平台没有可用的安全文件保存能力，导出内容未写入文件。',
      );
    } on Object {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.unexpected,
        message: '无法确认导出文件是否已保存，请检查所选位置后重试。',
      );
    } finally {
      bytes.fillRange(0, bytes.length, 0);
    }
  }

  @override
  Future<void> cancelPendingSave() async {
    try {
      await _channel.invokeMethod<bool>(_cancelMethod);
    } on PlatformException catch (exception) {
      throw _failureForPlatformCode(exception.code);
    } on MissingPluginException {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.unavailable,
        message: '当前平台无法停止进行中的导出保存，请检查所选位置。',
      );
    } on Object {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.unexpected,
        message: '无法确认进行中的导出保存是否已停止，请检查所选位置。',
      );
    }
  }

  Uint8List _encode(AccountDataExport export) {
    _validateExport(export);
    final String json;
    try {
      json = '${jsonEncode(export.toJson())}\n';
    } on Object {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.invalidPayload,
        message: '服务器返回的导出内容无法编码为 JSON，未写入文件。',
      );
    }
    final Uint8List bytes = Uint8List.fromList(utf8.encode(json));
    if (bytes.isEmpty) {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.invalidPayload,
        message: '服务器返回了空的导出内容，未写入文件。',
      );
    }
    if (bytes.length > _maxPayloadBytes) {
      bytes.fillRange(0, bytes.length, 0);
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.payloadTooLarge,
        message: '导出内容超过移动端 16 MiB 安全保存上限，未写入文件。请改用 Web 端下载。',
      );
    }
    return bytes;
  }

  void _validateExport(AccountDataExport export) {
    final Set<String> sections = export.includedSections.toSet();
    final List<Object> objectSections = <Object>[
      export.identity,
      export.forum,
      export.reviews,
      export.governance,
      export.credit,
      export.activity,
      export.platform,
    ];
    final bool hasExpectedSections =
        export.includedSections.length == _expectedSections.length &&
        sections.length == _expectedSections.length &&
        sections.containsAll(_expectedSections);
    final bool hasObjectPayloads = objectSections.every(_isStringKeyedMap);
    final bool hasObjectMedia = export.media.every(_isStringKeyedMap);
    if (export.schemaVersion != _expectedSchemaVersion ||
        export.generatedAt <= 0 ||
        !hasExpectedSections ||
        !hasObjectPayloads ||
        !hasObjectMedia) {
      throw const AccountExportSaveFailure(
        kind: AccountExportSaveFailureKind.invalidPayload,
        message: '服务器返回的导出格式与当前应用不兼容，未写入文件。请更新应用后重试。',
      );
    }
  }

  bool _isStringKeyedMap(Object value) {
    return value is Map && value.keys.every((Object? key) => key is String);
  }
}

AccountExportSaveFailure _failureForPlatformCode(String code) {
  return switch (code) {
    'EXPORT_BUSY' => const AccountExportSaveFailure(
      kind: AccountExportSaveFailureKind.busy,
      message: '已有一个导出保存窗口正在处理，请先完成或取消它。',
    ),
    'EXPORT_WRITE_FAILED' => const AccountExportSaveFailure(
      kind: AccountExportSaveFailureKind.writeFailed,
      message: '保存失败；所选位置可能留下不完整文件，请检查并删除后重试。',
    ),
    'EXPORT_CLEANUP_FAILED' => const AccountExportSaveFailure(
      kind: AccountExportSaveFailureKind.cleanupFailed,
      message: '系统可能已保存所选文件，但应用无法确认临时副本已清理。请完全关闭并重新打开应用后检查。',
    ),
    'EXPORT_UNAVAILABLE' ||
    'EXPORT_INTERRUPTED' => const AccountExportSaveFailure(
      kind: AccountExportSaveFailureKind.unavailable,
      message: '系统文件保存功能当前不可用，导出内容未确认写入。',
    ),
    'INVALID_ARGUMENT' => const AccountExportSaveFailure(
      kind: AccountExportSaveFailureKind.invalidPayload,
      message: '导出内容未通过本机安全校验，未写入文件。',
    ),
    _ => const AccountExportSaveFailure(
      kind: AccountExportSaveFailureKind.unexpected,
      message: '无法确认导出文件是否已保存，请检查所选位置后重试。',
    ),
  };
}
