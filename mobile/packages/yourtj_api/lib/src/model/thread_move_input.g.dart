// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_move_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadMoveInput _$ThreadMoveInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadMoveInput', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['boardId']);
      final val = ThreadMoveInput(
        boardId: $checkedConvert('boardId', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$ThreadMoveInputToJson(ThreadMoveInput instance) =>
    <String, dynamic>{'boardId': instance.boardId};
