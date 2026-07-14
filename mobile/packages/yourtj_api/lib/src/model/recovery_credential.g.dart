// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'recovery_credential.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RecoveryCredential _$RecoveryCredentialFromJson(Map<String, dynamic> json) =>
    $checkedCreate('RecoveryCredential', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['recoveryToken', 'expiresAt', 'lifecycle'],
      );
      final val = RecoveryCredential(
        recoveryToken: $checkedConvert('recoveryToken', (v) => v as String),
        expiresAt: $checkedConvert('expiresAt', (v) => (v as num).toInt()),
        lifecycle: $checkedConvert(
          'lifecycle',
          (v) => AccountLifecycle.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$RecoveryCredentialToJson(RecoveryCredential instance) =>
    <String, dynamic>{
      'recoveryToken': instance.recoveryToken,
      'expiresAt': instance.expiresAt,
      'lifecycle': instance.lifecycle.toJson(),
    };
