// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account_lifecycle.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AccountLifecycle _$AccountLifecycleFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AccountLifecycle', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'state',
          'deactivatedAt',
          'deletionRequestedAt',
          'recoverUntil',
          'deletedAt',
          'purgedAt',
          'lifecycleVersion',
        ],
      );
      final val = AccountLifecycle(
        state: $checkedConvert(
          'state',
          (v) => $enumDecode(
            _$AccountLifecycleStateEnumMap,
            v,
            unknownValue: AccountLifecycleState.unknownDefaultOpenApi,
          ),
        ),
        deactivatedAt: $checkedConvert(
          'deactivatedAt',
          (v) => (v as num?)?.toInt(),
        ),
        deletionRequestedAt: $checkedConvert(
          'deletionRequestedAt',
          (v) => (v as num?)?.toInt(),
        ),
        recoverUntil: $checkedConvert(
          'recoverUntil',
          (v) => (v as num?)?.toInt(),
        ),
        deletedAt: $checkedConvert('deletedAt', (v) => (v as num?)?.toInt()),
        purgedAt: $checkedConvert('purgedAt', (v) => (v as num?)?.toInt()),
        lifecycleVersion: $checkedConvert(
          'lifecycleVersion',
          (v) => (v as num).toInt(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$AccountLifecycleToJson(AccountLifecycle instance) =>
    <String, dynamic>{
      'state': _$AccountLifecycleStateEnumMap[instance.state]!,
      'deactivatedAt': instance.deactivatedAt,
      'deletionRequestedAt': instance.deletionRequestedAt,
      'recoverUntil': instance.recoverUntil,
      'deletedAt': instance.deletedAt,
      'purgedAt': instance.purgedAt,
      'lifecycleVersion': instance.lifecycleVersion,
    };

const _$AccountLifecycleStateEnumMap = {
  AccountLifecycleState.active: 'active',
  AccountLifecycleState.deactivated: 'deactivated',
  AccountLifecycleState.deletionRequested: 'deletion_requested',
  AccountLifecycleState.deleted: 'deleted',
  AccountLifecycleState.purged: 'purged',
  AccountLifecycleState.unknownDefaultOpenApi: 'unknown_default_open_api',
};
