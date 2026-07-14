// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'user_badge.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

UserBadge _$UserBadgeFromJson(Map<String, dynamic> json) =>
    $checkedCreate('UserBadge', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['slug', 'name']);
      final val = UserBadge(
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
      );
      return val;
    });

Map<String, dynamic> _$UserBadgeToJson(UserBadge instance) => <String, dynamic>{
  'slug': instance.slug,
  'name': instance.name,
};
