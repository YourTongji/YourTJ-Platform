// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'tag.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

Tag _$TagFromJson(Map<String, dynamic> json) => $checkedCreate('Tag', json, (
  $checkedConvert,
) {
  final val = Tag(
    id: $checkedConvert('id', (v) => v as String?),
    slug: $checkedConvert('slug', (v) => v as String?),
    name: $checkedConvert('name', (v) => v as String?),
    description: $checkedConvert('description', (v) => v as String?),
    threadCount: $checkedConvert('threadCount', (v) => (v as num?)?.toInt()),
    createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
  );
  return val;
});

Map<String, dynamic> _$TagToJson(Tag instance) => <String, dynamic>{
  'id': ?instance.id,
  'slug': ?instance.slug,
  'name': ?instance.name,
  'description': ?instance.description,
  'threadCount': ?instance.threadCount,
  'createdAt': ?instance.createdAt,
};
