// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account_data_export.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AccountDataExport _$AccountDataExportFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AccountDataExport', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'schemaVersion',
          'generatedAt',
          'includedSections',
          'identity',
          'forum',
          'reviews',
          'governance',
          'credit',
          'activity',
          'platform',
          'media',
        ],
      );
      final val = AccountDataExport(
        schemaVersion: $checkedConvert('schemaVersion', (v) => v as String),
        generatedAt: $checkedConvert('generatedAt', (v) => (v as num).toInt()),
        includedSections: $checkedConvert(
          'includedSections',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
        identity: $checkedConvert('identity', (v) => v as Object),
        forum: $checkedConvert('forum', (v) => v as Object),
        reviews: $checkedConvert('reviews', (v) => v as Object),
        governance: $checkedConvert('governance', (v) => v as Object),
        credit: $checkedConvert('credit', (v) => v as Object),
        activity: $checkedConvert('activity', (v) => v as Object),
        platform: $checkedConvert('platform', (v) => v as Object),
        media: $checkedConvert(
          'media',
          (v) => (v as List<dynamic>).map((e) => e as Object).toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$AccountDataExportToJson(AccountDataExport instance) =>
    <String, dynamic>{
      'schemaVersion': instance.schemaVersion,
      'generatedAt': instance.generatedAt,
      'includedSections': instance.includedSections,
      'identity': instance.identity,
      'forum': instance.forum,
      'reviews': instance.reviews,
      'governance': instance.governance,
      'credit': instance.credit,
      'activity': instance.activity,
      'platform': instance.platform,
      'media': instance.media,
    };
