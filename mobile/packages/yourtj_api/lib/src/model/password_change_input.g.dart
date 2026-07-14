// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'password_change_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PasswordChangeInput _$PasswordChangeInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PasswordChangeInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['currentPassword', 'newPassword']);
      final val = PasswordChangeInput(
        currentPassword: $checkedConvert('currentPassword', (v) => v as String),
        newPassword: $checkedConvert('newPassword', (v) => v as String),
        clientInstallationId: $checkedConvert(
          'clientInstallationId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$PasswordChangeInputToJson(
  PasswordChangeInput instance,
) => <String, dynamic>{
  'currentPassword': instance.currentPassword,
  'newPassword': instance.newPassword,
  'clientInstallationId': ?instance.clientInstallationId,
};
