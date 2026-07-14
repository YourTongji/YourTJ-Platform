// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'mod_action.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ModAction _$ModActionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ModAction', json, ($checkedConvert) {
      final val = ModAction(
        id: $checkedConvert('id', (v) => v as String?),
        actorId: $checkedConvert('actorId', (v) => v as String?),
        action: $checkedConvert('action', (v) => v as String?),
        targetType: $checkedConvert('targetType', (v) => v as String?),
        targetId: $checkedConvert('targetId', (v) => v as String?),
        reason: $checkedConvert('reason', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ModActionToJson(ModAction instance) => <String, dynamic>{
  'id': ?instance.id,
  'actorId': ?instance.actorId,
  'action': ?instance.action,
  'targetType': ?instance.targetType,
  'targetId': ?instance.targetId,
  'reason': ?instance.reason,
  'createdAt': ?instance.createdAt,
};
