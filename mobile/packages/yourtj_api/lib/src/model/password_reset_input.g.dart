// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'password_reset_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PasswordResetInput _$PasswordResetInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PasswordResetInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['email', 'code', 'newPassword']);
      final val = PasswordResetInput(
        email: $checkedConvert('email', (v) => v as String),
        code: $checkedConvert('code', (v) => v as String),
        newPassword: $checkedConvert('newPassword', (v) => v as String),
        clientInstallationId: $checkedConvert(
          'clientInstallationId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$PasswordResetInputToJson(PasswordResetInput instance) =>
    <String, dynamic>{
      'email': instance.email,
      'code': instance.code,
      'newPassword': instance.newPassword,
      'clientInstallationId': ?instance.clientInstallationId,
    };
