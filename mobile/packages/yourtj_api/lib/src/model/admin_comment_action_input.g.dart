// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_comment_action_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminCommentActionInput _$AdminCommentActionInputFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('AdminCommentActionInput', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['reason']);
  final val = AdminCommentActionInput(
    reason: $checkedConvert('reason', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$AdminCommentActionInputToJson(
  AdminCommentActionInput instance,
) => <String, dynamic>{'reason': instance.reason};
