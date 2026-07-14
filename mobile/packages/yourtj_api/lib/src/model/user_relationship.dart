//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:json_annotation/json_annotation.dart';

part 'user_relationship.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class UserRelationship {
  /// Returns a new [UserRelationship] instance.
  UserRelationship({
    required this.isSelf,

    required this.following,

    required this.followedBy,

    required this.muted,

    required this.blockedByMe,

    required this.blockedMe,

    required this.canFollow,

    required this.canStartConversation,

    required this.canMention,
  });

  @JsonKey(name: r'isSelf', required: true, includeIfNull: false)
  final bool isSelf;

  @JsonKey(name: r'following', required: true, includeIfNull: false)
  final bool following;

  @JsonKey(name: r'followedBy', required: true, includeIfNull: false)
  final bool followedBy;

  @JsonKey(name: r'muted', required: true, includeIfNull: false)
  final bool muted;

  @JsonKey(name: r'blockedByMe', required: true, includeIfNull: false)
  final bool blockedByMe;

  @JsonKey(name: r'blockedMe', required: true, includeIfNull: false)
  final bool blockedMe;

  @JsonKey(name: r'canFollow', required: true, includeIfNull: false)
  final bool canFollow;

  /// Whether the current account may start a new conversation under block and recipient DM policy.
  @JsonKey(name: r'canStartConversation', required: true, includeIfNull: false)
  final bool canStartConversation;

  /// Whether the current account may create a semantic mention notification under block and recipient mention policy.
  @JsonKey(name: r'canMention', required: true, includeIfNull: false)
  final bool canMention;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is UserRelationship &&
          other.isSelf == isSelf &&
          other.following == following &&
          other.followedBy == followedBy &&
          other.muted == muted &&
          other.blockedByMe == blockedByMe &&
          other.blockedMe == blockedMe &&
          other.canFollow == canFollow &&
          other.canStartConversation == canStartConversation &&
          other.canMention == canMention;

  @override
  int get hashCode =>
      isSelf.hashCode +
      following.hashCode +
      followedBy.hashCode +
      muted.hashCode +
      blockedByMe.hashCode +
      blockedMe.hashCode +
      canFollow.hashCode +
      canStartConversation.hashCode +
      canMention.hashCode;

  factory UserRelationship.fromJson(Map<String, dynamic> json) =>
      _$UserRelationshipFromJson(json);

  Map<String, dynamic> toJson() => _$UserRelationshipToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
