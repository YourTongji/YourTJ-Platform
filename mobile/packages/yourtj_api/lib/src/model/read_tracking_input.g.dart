// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'read_tracking_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ReadTrackingInput _$ReadTrackingInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ReadTrackingInput', json, ($checkedConvert) {
      final val = ReadTrackingInput(
        lastReadCommentId: $checkedConvert(
          'lastReadCommentId',
          (v) => v as String?,
        ),
      );
      return val;
    });

Map<String, dynamic> _$ReadTrackingInputToJson(ReadTrackingInput instance) =>
    <String, dynamic>{'lastReadCommentId': ?instance.lastReadCommentId};
