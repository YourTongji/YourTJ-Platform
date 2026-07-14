// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'forum_polls_id_vote_post_request.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ForumPollsIdVotePostRequest _$ForumPollsIdVotePostRequestFromJson(
  Map<String, dynamic> json,
) => $checkedCreate('ForumPollsIdVotePostRequest', json, ($checkedConvert) {
  $checkKeys(json, requiredKeys: const ['optionId']);
  final val = ForumPollsIdVotePostRequest(
    optionId: $checkedConvert('optionId', (v) => v as String),
  );
  return val;
});

Map<String, dynamic> _$ForumPollsIdVotePostRequestToJson(
  ForumPollsIdVotePostRequest instance,
) => <String, dynamic>{'optionId': instance.optionId};
