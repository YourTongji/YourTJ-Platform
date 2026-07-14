//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/profile_visibility.dart';
import 'package:yourtj_api/src/model/activity_visibility.dart';
import 'package:yourtj_api/src/model/mention_policy.dart';
import 'package:yourtj_api/src/model/dm_policy.dart';
import 'package:yourtj_api/src/model/relationship_list_visibility.dart';
import 'package:json_annotation/json_annotation.dart';

part 'profile_privacy_update_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class ProfilePrivacyUpdateInput {
  /// Returns a new [ProfilePrivacyUpdateInput] instance.
  ProfilePrivacyUpdateInput({
    required this.profileVisibility,

    this.activityVisibility,

    required this.followersVisibility,

    required this.followingVisibility,

    required this.discoverable,

    required this.dmPolicy,

    this.mentionPolicy,
  });

  @JsonKey(
    name: r'profileVisibility',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ProfileVisibility.unknownDefaultOpenApi,
  )
  final ProfileVisibility profileVisibility;

  @JsonKey(
    name: r'activityVisibility',
    required: false,
    includeIfNull: false,
    unknownEnumValue: ActivityVisibility.unknownDefaultOpenApi,
  )
  final ActivityVisibility? activityVisibility;

  @JsonKey(
    name: r'followersVisibility',
    required: true,
    includeIfNull: false,
    unknownEnumValue: RelationshipListVisibility.unknownDefaultOpenApi,
  )
  final RelationshipListVisibility followersVisibility;

  @JsonKey(
    name: r'followingVisibility',
    required: true,
    includeIfNull: false,
    unknownEnumValue: RelationshipListVisibility.unknownDefaultOpenApi,
  )
  final RelationshipListVisibility followingVisibility;

  @JsonKey(name: r'discoverable', required: true, includeIfNull: false)
  final bool discoverable;

  @JsonKey(
    name: r'dmPolicy',
    required: true,
    includeIfNull: false,
    unknownEnumValue: DmPolicy.unknownDefaultOpenApi,
  )
  final DmPolicy dmPolicy;

  @JsonKey(
    name: r'mentionPolicy',
    required: false,
    includeIfNull: false,
    unknownEnumValue: MentionPolicy.unknownDefaultOpenApi,
  )
  final MentionPolicy? mentionPolicy;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is ProfilePrivacyUpdateInput &&
          other.profileVisibility == profileVisibility &&
          other.activityVisibility == activityVisibility &&
          other.followersVisibility == followersVisibility &&
          other.followingVisibility == followingVisibility &&
          other.discoverable == discoverable &&
          other.dmPolicy == dmPolicy &&
          other.mentionPolicy == mentionPolicy;

  @override
  int get hashCode =>
      profileVisibility.hashCode +
      activityVisibility.hashCode +
      followersVisibility.hashCode +
      followingVisibility.hashCode +
      discoverable.hashCode +
      dmPolicy.hashCode +
      mentionPolicy.hashCode;

  factory ProfilePrivacyUpdateInput.fromJson(Map<String, dynamic> json) =>
      _$ProfilePrivacyUpdateInputFromJson(json);

  Map<String, dynamic> toJson() => _$ProfilePrivacyUpdateInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
