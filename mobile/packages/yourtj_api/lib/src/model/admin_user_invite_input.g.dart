// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_user_invite_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminUserInviteInput _$AdminUserInviteInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminUserInviteInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['email', 'handle', 'reason']);
  final val = AdminUserInviteInput(
    email: $checkedConvert('email', (v) => v as String),
    handle: $checkedConvert('handle', (v) => v as String),
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminUserInviteInputToJson(
  AdminUserInviteInput instance,
) => <String, dynamic>{
  'email': instance.email,
  'handle': instance.handle,
  'reason': instance.reason,
};
