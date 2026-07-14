//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/profile_visibility.dart';
import 'package:yourtj_api/src/model/activity_visibility.dart';
import 'package:json_annotation/json_annotation.dart';

part 'onboarding_complete_input.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class OnboardingCompleteInput {
  /// Returns a new [OnboardingCompleteInput] instance.
  OnboardingCompleteInput({
    required this.handle,

    required this.displayName,

    required this.bio,

    required this.profileVisibility,

    required this.activityVisibility,

    required this.discoverable,

    required this.acceptedTermsVersion,
  });

  @JsonKey(name: r'handle', required: true, includeIfNull: false)
  final String handle;

  @JsonKey(name: r'displayName', required: true, includeIfNull: true)
  final String? displayName;

  @JsonKey(name: r'bio', required: true, includeIfNull: true)
  final String? bio;

  @JsonKey(
    name: r'profileVisibility',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ProfileVisibility.unknownDefaultOpenApi,
  )
  final ProfileVisibility profileVisibility;

  @JsonKey(
    name: r'activityVisibility',
    required: true,
    includeIfNull: false,
    unknownEnumValue: ActivityVisibility.unknownDefaultOpenApi,
  )
  final ActivityVisibility activityVisibility;

  @JsonKey(name: r'discoverable', required: true, includeIfNull: false)
  final bool discoverable;

  @JsonKey(name: r'acceptedTermsVersion', required: true, includeIfNull: false)
  final String acceptedTermsVersion;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is OnboardingCompleteInput &&
          other.handle == handle &&
          other.displayName == displayName &&
          other.bio == bio &&
          other.profileVisibility == profileVisibility &&
          other.activityVisibility == activityVisibility &&
          other.discoverable == discoverable &&
          other.acceptedTermsVersion == acceptedTermsVersion;

  @override
  int get hashCode =>
      handle.hashCode +
      (displayName == null ? 0 : displayName.hashCode) +
      (bio == null ? 0 : bio.hashCode) +
      profileVisibility.hashCode +
      activityVisibility.hashCode +
      discoverable.hashCode +
      acceptedTermsVersion.hashCode;

  factory OnboardingCompleteInput.fromJson(Map<String, dynamic> json) =>
      _$OnboardingCompleteInputFromJson(json);

  Map<String, dynamic> toJson() => _$OnboardingCompleteInputToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
