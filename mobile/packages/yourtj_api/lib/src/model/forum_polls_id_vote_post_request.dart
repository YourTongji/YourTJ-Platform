//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'forum_polls_id_vote_post_request.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ForumPollsIdVotePostRequest {
  /// Returns a new [ForumPollsIdVotePostRequest] instance.
  ForumPollsIdVotePostRequest({required this.optionId});

  @JsonKey(name: r'optionId', required: true, includeIfNull: false)
  final String optionId;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ForumPollsIdVotePostRequest && other.optionId == optionId;

  @override
  int get hashCode => optionId.hashCode;

  factory ForumPollsIdVotePostRequest.fromJson(Map<String, dynamic> json) =>
      _$ForumPollsIdVotePostRequestFromJson(json);

  Map<String, dynamic> toJson() => _$ForumPollsIdVotePostRequestToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
