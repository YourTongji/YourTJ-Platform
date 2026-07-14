// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'draft_output.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DraftOutput _$DraftOutputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DraftOutput', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['draftKey', 'payload', 'version', 'updatedAt'],
      );
      final val = DraftOutput(
        draftKey: $checkedConvert('draftKey', (v) => v as String),
        payload: $checkedConvert(
          'payload',
          (v) => ForumDraftPayload.fromJson(v as Map<String, dynamic>),
        ),
        version: $checkedConvert('version', (v) => (v as num).toInt()),
        updatedAt: $checkedConvert('updatedAt', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$DraftOutputToJson(DraftOutput instance) =>
    <String, dynamic>{
      'draftKey': instance.draftKey,
      'payload': instance.payload.toJson(),
      'version': instance.version,
      'updatedAt': instance.updatedAt,
    };
