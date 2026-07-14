// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'flag_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

FlagResponse _$FlagResponseFromJson(Map<String, dynamic> json) =>
    $checkedCreate('FlagResponse', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const ['ok', 'autoHidden', 'autoSilenced'],
      );
      final val = FlagResponse(
        ok: $checkedConvert('ok', (v) => v as bool),
        autoHidden: $checkedConvert('autoHidden', (v) => v as bool),
        autoSilenced: $checkedConvert('autoSilenced', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$FlagResponseToJson(FlagResponse instance) =>
    <String, dynamic>{
      'ok': instance.ok,
      'autoHidden': instance.autoHidden,
      'autoSilenced': instance.autoSilenced,
    };
