// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'poll_vote_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

PollVoteResponse _$PollVoteResponseFromJson(Map<String, dynamic> json) =>
    $checkedCreate('PollVoteResponse', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['ok', 'myVotes']);
      final val = PollVoteResponse(
        ok: $checkedConvert('ok', (v) => v as bool),
        myVotes: $checkedConvert(
          'myVotes',
          (v) => (v as List<dynamic>).map((e) => e as String).toList(),
        ),
      );
      return val;
    });

Map<String, dynamic> _$PollVoteResponseToJson(PollVoteResponse instance) =>
    <String, dynamic>{'ok': instance.ok, 'myVotes': instance.myVotes};
