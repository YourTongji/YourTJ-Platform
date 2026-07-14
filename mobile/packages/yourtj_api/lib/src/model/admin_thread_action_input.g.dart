// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_thread_action_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminThreadActionInput _$AdminThreadActionInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminThreadActionInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = AdminThreadActionInput(
    reason: $checkedConvert('reason', (v) => v as String),
    globally: $checkedConvert('globally', (v) => v as bool?),
    boardId: $checkedConvert('boardId', (v) => v as String?),
  );
  return val;
});

Map<String, dynamic> _$AdminThreadActionInputToJson(
  AdminThreadActionInput instance,
) => <String, dynamic>{
  'reason': instance.reason,
  'globally': ?instance.globally,
  'boardId': ?instance.boardId,
};
