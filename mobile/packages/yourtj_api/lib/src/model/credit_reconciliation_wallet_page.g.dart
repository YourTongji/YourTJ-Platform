// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'credit_reconciliation_wallet_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

CreditReconciliationWalletPage _$CreditReconciliationWalletPageFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('CreditReconciliationWalletPage', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
  final val = CreditReconciliationWalletPage(
    items: $checkedConvert(
      'items',
      (v) => (v as List<dynamic>)
          .map(
            (e) =>
                CreditReconciliationWallet.fromJson(e as Map<String, dynamic>),
          )
          .toList(),
    ),
    nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
    hasMore: $checkedConvert('hasMore', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$CreditReconciliationWalletPageToJson(
  CreditReconciliationWalletPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
