// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'forum_threads_id_delete200_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ForumThreadsIdDelete200Response _$ForumThreadsIdDelete200ResponseFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ForumThreadsIdDelete200Response', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['ok']);
  final val = ForumThreadsIdDelete200Response(
    ok: $checkedConvert('ok', (v) => v as bool),
  );
  return val;
});

Map<String, dynamic> _$ForumThreadsIdDelete200ResponseToJson(
  ForumThreadsIdDelete200Response instance,
) => <String, dynamic>{'ok': instance.ok};
