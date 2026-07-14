//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'vote_response.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class VoteResponse {
  /// Returns a new [VoteResponse] instance.
  VoteResponse({
    required this.ok,

    required this.voteCount,

    required this.viewerVote,
  });

  @JsonKey(name: r'ok', required: true, includeIfNull: false)
  final bool ok;

  @JsonKey(name: r'voteCount', required: true, includeIfNull: false)
  final int voteCount;

  @JsonKey(
    name: r'viewerVote',
    required: true,
    includeIfNull: true,
    unknownEnumValue: VoteResponseViewerVoteEnum.unknownDefaultOpenApi,
  )
  final VoteResponseViewerVoteEnum? viewerVote;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is VoteResponse &&
          other.ok == ok &&
          other.voteCount == voteCount &&
          other.viewerVote == viewerVote;

  @override
  int get hashCode =>
      ok.hashCode +
      voteCount.hashCode +
      (viewerVote == null ? 0 : viewerVote.hashCode);

  factory VoteResponse.fromJson(Map<String, dynamic> json) =>
      _$VoteResponseFromJson(json);

  Map<String, dynamic> toJson() => _$VoteResponseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}

enum VoteResponseViewerVoteEnum {
  @JsonValue(r'up')
  up(r'up'),
  @JsonValue(r'down')
  down(r'down'),
  @JsonValue(r'unknown_default_open_api')
  unknownDefaultOpenApi(r'unknown_default_open_api');

  const VoteResponseViewerVoteEnum(this.value);

  final String value;

  @override
  String toString() => value;
}
