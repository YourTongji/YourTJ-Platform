// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'draft_save_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DraftSaveInput _$DraftSaveInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DraftSaveInput', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['draftKey', 'expectedVersion', 'payload'],
      );
      final val = DraftSaveInput(
        draftKey: $checkedConvert('draftKey', (v) => v as String),
        expectedVersion: $checkedConvert(
          'expectedVersion',
          (v) => (v as num).toInt(),
        ),
        payload: $checkedConvert(
          'payload',
          (v) => ForumDraftPayload.fromJson(v as Map<String, dynamic>),
        ),
      );
      return val;
    });

Map<String, dynamic> _$DraftSaveInputToJson(DraftSaveInput instance) =>
    <String, dynamic>{
      'draftKey': instance.draftKey,
      'expectedVersion': instance.expectedVersion,
      'payload': instance.payload.toJson(),
    };
