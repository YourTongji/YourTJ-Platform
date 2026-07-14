// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'search_highlight_range.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

SearchHighlightRange _$SearchHighlightRangeFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('SearchHighlightRange', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['start', 'end']);
  final val = SearchHighlightRange(
    start: $checkedConvert('start', (v) => (v as num).toInt()),
    end: $checkedConvert('end', (v) => (v as num).toInt()),
  );
  return val;
});

Map<String, dynamic> _$SearchHighlightRangeToJson(
  SearchHighlightRange instance,
) => <String, dynamic>{'start': instance.start, 'end': instance.end};
