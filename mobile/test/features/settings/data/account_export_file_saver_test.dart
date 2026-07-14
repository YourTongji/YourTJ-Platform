import 'dart:convert';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:yourtj_api/yourtj_api.dart';
import 'package:yourtj_mobile/features/settings/data/account_export_file_saver.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const MethodChannel channel = MethodChannel(
    'de.yourtj.mobile/account-export.test',
  );

  tearDown(() async {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test('sends only the fixed filename and bounded UTF-8 JSON bytes', () async {
    MethodCall? receivedCall;
    Uint8List? receivedBytes;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (MethodCall call) async {
          receivedCall = call;
          final Map<Object?, Object?> arguments =
              call.arguments! as Map<Object?, Object?>;
          receivedBytes = Uint8List.fromList(arguments['bytes']! as Uint8List);
          return 'saved';
        });
    final MethodChannelAccountExportFileSaver saver =
        MethodChannelAccountExportFileSaver(channel: channel);

    expect(await saver.save(_export()), AccountExportSaveResult.saved);

    expect(receivedCall?.method, 'saveAccountExport');
    final Map<Object?, Object?> arguments =
        receivedCall!.arguments! as Map<Object?, Object?>;
    expect(arguments['fileName'], accountExportFileName);
    final Object? decoded = jsonDecode(utf8.decode(receivedBytes!));
    expect(decoded, isA<Map<String, Object?>>());
    expect(
      (decoded! as Map<String, Object?>)['schemaVersion'],
      'yourtj.account-export.v2',
    );
  });

  test(
    'exposes explicit platform cancellation without treating it as success',
    () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            expect(call.method, 'saveAccountExport');
            return 'cancelled';
          });
      final MethodChannelAccountExportFileSaver saver =
          MethodChannelAccountExportFileSaver(channel: channel);

      expect(await saver.save(_export()), AccountExportSaveResult.cancelled);
    },
  );

  test(
    'uses the narrow cancellation method for a pending native picker',
    () async {
      MethodCall? receivedCall;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            receivedCall = call;
            return true;
          });
      final MethodChannelAccountExportFileSaver saver =
          MethodChannelAccountExportFileSaver(channel: channel);

      await saver.cancelPendingSave();

      expect(receivedCall?.method, 'cancelAccountExport');
      expect(receivedCall?.arguments, isNull);
    },
  );

  test(
    'rejects an incompatible API payload before invoking native code',
    () async {
      int nativeCalls = 0;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            nativeCalls += 1;
            return 'saved';
          });
      final MethodChannelAccountExportFileSaver saver =
          MethodChannelAccountExportFileSaver(channel: channel);

      await expectLater(
        saver.save(_export(schemaVersion: 'yourtj.account-export.v3')),
        throwsA(
          isA<AccountExportSaveFailure>().having(
            (AccountExportSaveFailure failure) => failure.kind,
            'kind',
            AccountExportSaveFailureKind.invalidPayload,
          ),
        ),
      );
      expect(nativeCalls, 0);
    },
  );

  test(
    'rejects UTF-8 payloads over the configured closed test bound',
    () async {
      int nativeCalls = 0;
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            nativeCalls += 1;
            return 'saved';
          });
      final MethodChannelAccountExportFileSaver saver =
          MethodChannelAccountExportFileSaver(
            channel: channel,
            maxPayloadBytes: 256,
          );

      await expectLater(
        saver.save(_export(identity: <String, Object?>{'value': 'x' * 512})),
        throwsA(
          isA<AccountExportSaveFailure>().having(
            (AccountExportSaveFailure failure) => failure.kind,
            'kind',
            AccountExportSaveFailureKind.payloadTooLarge,
          ),
        ),
      );
      expect(nativeCalls, 0);
    },
  );

  test(
    'does not expose native paths in a fixed platform failure message',
    () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (MethodCall call) async {
            throw PlatformException(
              code: 'EXPORT_WRITE_FAILED',
              message: 'file:///private/secret/account.json',
            );
          });
      final MethodChannelAccountExportFileSaver saver =
          MethodChannelAccountExportFileSaver(channel: channel);

      await expectLater(
        saver.save(_export()),
        throwsA(
          isA<AccountExportSaveFailure>()
              .having(
                (AccountExportSaveFailure failure) => failure.kind,
                'kind',
                AccountExportSaveFailureKind.writeFailed,
              )
              .having(
                (AccountExportSaveFailure failure) => failure.message,
                'message',
                isNot(contains('/private/secret')),
              ),
        ),
      );
    },
  );
}

AccountDataExport _export({
  String schemaVersion = 'yourtj.account-export.v2',
  Object identity = const <String, Object?>{'account': <String, Object?>{}},
}) {
  return AccountDataExport(
    schemaVersion: schemaVersion,
    generatedAt: 100,
    includedSections: const <String>[
      'identity',
      'forum',
      'reviews',
      'governance',
      'credit',
      'activity',
      'platform',
      'mediaMetadata',
    ],
    identity: identity,
    forum: const <String, Object?>{},
    reviews: const <String, Object?>{},
    governance: const <String, Object?>{},
    credit: const <String, Object?>{},
    activity: const <String, Object?>{},
    platform: const <String, Object?>{},
    media: const <Object>[],
  );
}
