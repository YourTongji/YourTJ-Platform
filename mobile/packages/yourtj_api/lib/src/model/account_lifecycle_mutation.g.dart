// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account_lifecycle_mutation.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AccountLifecycleMutation _$AccountLifecycleMutationFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AccountLifecycleMutation', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['lifecycle', 'recovery']);
  final val = AccountLifecycleMutation(
    lifecycle: $checkedConvert(
      'lifecycle',
      (v) => AccountLifecycle.fromJson(v as Map<String, dynamic>),
    ),
    recovery: $checkedConvert(
      'recovery',
      (v) => RecoveryCredential.fromJson(v as Map<String, dynamic>),
    ),
  );
  return val;
});

Map<String, dynamic> _$AccountLifecycleMutationToJson(
  AccountLifecycleMutation instance,
) => <String, dynamic>{
  'lifecycle': instance.lifecycle.toJson(),
  'recovery': instance.recovery.toJson(),
};
