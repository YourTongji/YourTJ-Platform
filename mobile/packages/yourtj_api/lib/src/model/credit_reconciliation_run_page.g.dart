// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credit_reconciliation_run_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CreditReconciliationRunPage _$CreditReconciliationRunPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('CreditReconciliationRunPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = CreditReconciliationRunPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map(
            (e) => CreditReconciliationRun.fromJson(e as Map<String, dynamic>),
          )
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$CreditReconciliationRunPageToJson(
  CreditReconciliationRunPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
