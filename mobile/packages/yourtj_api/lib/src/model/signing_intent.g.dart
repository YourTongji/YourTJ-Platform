// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'signing_intent.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SigningIntent _$SigningIntentFromJson(Map<String, dynamic> json) =>
    $checkedCreate('SigningIntent', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['intentId', 'signingBytes', 'expiresAt'],
      );
      final val = SigningIntent(
        intentId: $checkedConvert('intentId', (v) => v as String),
        signingBytes: $checkedConvert('signingBytes', (v) => v as String),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$SigningIntentToJson(SigningIntent instance) =>
    <String, dynamic>{
      'intentId': instance.intentId,
      'signingBytes': instance.signingBytes,
      'expiresAt': instance.expiresAt,
    };
