// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'onebox_result.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

OneboxResult _$OneboxResultFromJson(Map<String, dynamic> json) =>
    $checkedCreate('OneboxResult', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'type',
          'url',
          'title',
          'description',
          'imageUrl',
          'siteName',
        ],
      );
      final val = OneboxResult(
        type: $checkedConvert(
          'type',
          (v) => $enumDecode(
            _$OneboxResultTypeEnumEnumMap,
            v,
            unknownValue: OneboxResultTypeEnum.unknownDefaultOpenApi,
          ),
        ),
        url: $checkedConvert('url', (v) => v as String),
        title: $checkedConvert('title', (v) => v as String?),
        description: $checkedConvert('description', (v) => v as String?),
        imageUrl: $checkedConvert('imageUrl', (v) => v as String?),
        siteName: $checkedConvert('siteName', (v) => v as String?),
      );
      return val;
    });

Map<String, dynamic> _$OneboxResultToJson(OneboxResult instance) =>
    <String, dynamic>{
      'type': _$OneboxResultTypeEnumEnumMap[instance.type]!,
      'url': instance.url,
      'title': instance.title,
      'description': instance.description,
      'imageUrl': instance.imageUrl,
      'siteName': instance.siteName,
    };

const _$OneboxResultTypeEnumEnumMap = {
  OneboxResultTypeEnum.plain: 'plain',
  OneboxResultTypeEnum.card: 'card',
  OneboxResultTypeEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
