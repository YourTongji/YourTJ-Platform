// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'dm_report_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

DmReportPage _$DmReportPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('DmReportPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = DmReportPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => DmReport.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$DmReportPageToJson(DmReportPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
