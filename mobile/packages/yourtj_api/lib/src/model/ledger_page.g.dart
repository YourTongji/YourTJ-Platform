// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ledger_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

LedgerPage _$LedgerPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('LedgerPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = LedgerPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => LedgerEntry.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$LedgerPageToJson(LedgerPage instance) =>
    <String, dynamic>{
      'items': instance.items.map((e) => e.toJson()).toList(),
      'nextCursor': instance.nextCursor,
      'hasMore': instance.hasMore,
    };
