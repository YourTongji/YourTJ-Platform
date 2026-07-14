// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'poll_option.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PollOption _$PollOptionFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PollOption', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['id', 'label', 'voteCount', 'position'],
      );
      final val = PollOption(
        id: $checkedConvert('id', (v) => v as String),
        label: $checkedConvert('label', (v) => v as String),
        voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
        position: $checkedConvert('position', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$PollOptionToJson(PollOption instance) =>
    <String, dynamic>{
      'id': instance.id,
      'label': instance.label,
      'voteCount': instance.voteCount,
      'position': instance.position,
    };
