// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'password_set_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PasswordSetInput _$PasswordSetInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PasswordSetInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['newPassword']);
      final val = PasswordSetInput(
        newPassword: $checkedConvert('newPassword', (v) => v as String),
        clientInstallationId: $checkedConvert(
          'clientInstallationId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$PasswordSetInputToJson(PasswordSetInput instance) =>
    <String, dynamic>{
      'newPassword': instance.newPassword,
      'clientInstallationId': ?instance.clientInstallationId,
    };
