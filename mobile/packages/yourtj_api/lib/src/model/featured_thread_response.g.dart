// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'featured_thread_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

FeaturedThreadResponse _$FeaturedThreadResponseFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('FeaturedThreadResponse', json, ($checkedConvert) {
  final val = FeaturedThreadResponse(
    id: $checkedConvert('id', (v) => v as String?),
    featuredAt: $checkedConvert('featuredAt', (v) => (v as num?)?.toInt()),
  );
  return val;
});

Map<String, dynamic> _$FeaturedThreadResponseToJson(
  FeaturedThreadResponse instance,
) => <String, dynamic>{'id': ?instance.id, 'featuredAt': ?instance.featuredAt};
