// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'vote_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

VoteResponse _$VoteResponseFromJson(Map<String, dynamic> json) =>
    $checkedCreate('VoteResponse', json, ($checkedConvert) {
      $checkKeys(json, requiredKeys: const ['ok', 'voteCount', 'viewerVote']);
      final val = VoteResponse(
        ok: $checkedConvert('ok', (v) => v as bool),
        voteCount: $checkedConvert('voteCount', (v) => (v as num).toInt()),
        viewerVote: $checkedConvert(
          'viewerVote',
          (v) => $enumDecodeNullable(
            _$VoteResponseViewerVoteEnumEnumMap,
            v,
            unknownValue: VoteResponseViewerVoteEnum.unknownDefaultOpenApi,
          ),
        ),
      );
      return val;
    });

Map<String, dynamic> _$VoteResponseToJson(VoteResponse instance) =>
    <String, dynamic>{
      'ok': instance.ok,
      'voteCount': instance.voteCount,
      'viewerVote': _$VoteResponseViewerVoteEnumEnumMap[instance.viewerVote],
    };

const _$VoteResponseViewerVoteEnumEnumMap = {
  VoteResponseViewerVoteEnum.up: 'up',
  VoteResponseViewerVoteEnum.down: 'down',
  VoteResponseViewerVoteEnum.unknownDefaultOpenApi: 'unknown_default_open_api',
};
