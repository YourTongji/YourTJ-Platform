// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'admin_audit_event_page.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AdminAuditEventPage _$AdminAuditEventPageFromJson(Map<String, dynamic> json) =>
    $checkedCreate('AdminAuditEventPage', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['items', 'nextCursor', 'hasMore']);
      final val = AdminAuditEventPage(
        items: $checkedConvert(
          'items',
          (v) => (v as List<dynamic>)
              .map((e) => AdminAuditEvent.fromJson(e as Map<String, dynamic>))
              .toList(),
        ),
        nextCursor: $checkedConvert('nextCursor', (v) => v as String?),
        hasMore: $checkedConvert('hasMore', (v) => v as bool),
      );
      return val;
    });

Map<String, dynamic> _$AdminAuditEventPageToJson(
  AdminAuditEventPage instance,
) => <String, dynamic>{
  'items': instance.items.map((e) => e.toJson()).toList(),
  'nextCursor': instance.nextCursor,
  'hasMore': instance.hasMore,
};
