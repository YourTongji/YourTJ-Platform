// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'activity_weights.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ActivityWeights _$ActivityWeightsFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ActivityWeights', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['thread', 'comment', 'like', 'checkIn'],
      );
      final val = ActivityWeights(
        thread: $checkedConvert('thread', (v) => (v as num).toInt()),
        comment: $checkedConvert('comment', (v) => (v as num).toInt()),
        like: $checkedConvert('like', (v) => (v as num).toInt()),
        checkIn: $checkedConvert('checkIn', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$ActivityWeightsToJson(ActivityWeights instance) =>
    <String, dynamic>{
      'thread': instance.thread,
      'comment': instance.comment,
      'like': instance.like,
      'checkIn': instance.checkIn,
    };
