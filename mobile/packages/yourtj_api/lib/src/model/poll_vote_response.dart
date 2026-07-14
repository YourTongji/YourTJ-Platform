//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'poll_vote_response.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class PollVoteResponse {
  /// Returns a new [PollVoteResponse] instance.
  PollVoteResponse({required this.ok, required this.myVotes});

  @JsonKey(name: r'ok', required: true, includeIfNull: false)
  final bool ok;

  @JsonKey(name: r'myVotes', required: true, includeIfNull: false)
  final List<String> myVotes;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is PollVoteResponse && other.ok == ok && other.myVotes == myVotes;

  @override
  int get hashCode => ok.hashCode + myVotes.hashCode;

  factory PollVoteResponse.fromJson(Map<String, dynamic> json) =>
      _$PollVoteResponseFromJson(json);

  Map<String, dynamic> toJson() => _$PollVoteResponseToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
