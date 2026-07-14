// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'board_search_hit.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BoardSearchHit _$BoardSearchHitFromJson(Map<String, dynamic> json) =>
    $checkedCreate('BoardSearchHit', json, ($checkedConvert) {
      $checkKeys(
        json,
        requiredKeys: const [
          'id',
          'slug',
          'name',
          'description',
          'threadCount',
        ],
      );
      final val = BoardSearchHit(
        id: $checkedConvert('id', (v) => v as String),
        slug: $checkedConvert('slug', (v) => v as String),
        name: $checkedConvert('name', (v) => v as String),
        description: $checkedConvert('description', (v) => v as String?),
        threadCount: $checkedConvert('threadCount', (v) => (v as num).toInt()),
      );
      return val;
    });

Map<String, dynamic> _$BoardSearchHitToJson(BoardSearchHit instance) =>
    <String, dynamic>{
      'id': instance.id,
      'slug': instance.slug,
      'name': instance.name,
      'description': instance.description,
      'threadCount': instance.threadCount,
    };
