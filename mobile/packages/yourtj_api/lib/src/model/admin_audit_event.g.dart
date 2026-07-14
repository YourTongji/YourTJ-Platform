// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_audit_event.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminAuditEvent _$AdminAuditEventFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminAuditEvent', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'actorKind',
          'action',
          'targetType',
          'targetId',
          'createdAt',
        ],
      );
      final val = AdminAuditEvent(
        id: $checkedConvert('id', (v) => v as String),
        actorKind: $checkedConvert(
          'actorKind',
          (v) => $enumDecode(
            _$AdminAuditEventActorKindEnumEnumMap,
            v,
            unknownValue: AdminAuditEventActorKindEnum.unknownDefaultOpenApi,
          ),
        ),
        actorId: $checkedConvert('actorId', (v) => v as String?),
        actorHandle: $checkedConvert('actorHandle', (v) => v as String?),
        actorRole: $checkedConvert('actorRole', (v) => v as String?),
        action: $checkedConvert('action', (v) => v as String),
        targetType: $checkedConvert('targetType', (v) => v as String),
        targetId: $checkedConvert('targetId', (v) => v as String),
        reason: $checkedConvert('reason', (v) => v as String?),
        metadata: $checkedConvert('metadata', (v) => v),
        createdAt: $checkedConvert('createdAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$AdminAuditEventToJson(AdminAuditEvent instance) =>
    <String, dynamic>{
      'id': instance.id,
      'actorKind': _$AdminAuditEventActorKindEnumEnumMap[instance.actorKind]!,
      'actorId': ?instance.actorId,
      'actorHandle': ?instance.actorHandle,
      'actorRole': ?instance.actorRole,
      'action': instance.action,
      'targetType': instance.targetType,
      'targetId': instance.targetId,
      'reason': ?instance.reason,
      'metadata': ?instance.metadata,
      'createdAt': instance.createdAt,
    };

const _$AdminAuditEventActorKindEnumEnumMap = {
  AdminAuditEventActorKindEnum.account: 'account',
  AdminAuditEventActorKindEnum.system: 'system',
  AdminAuditEventActorKindEnum.service: 'service',
  AdminAuditEventActorKindEnum.unknownDefaultOpenApi:
      'unknown_default_open_api',
};
