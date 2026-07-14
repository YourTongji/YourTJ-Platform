//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/profile_visibility.dart';
import 'package:yourtj_api/src/model/activity_visibility.dart';
import 'package:json_annotation/json_annotation.dart';

part 'onboarding_state.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class OnboardingState {
  /// Returns a new [OnboardingState] instance.
  OnboardingState({
    required this.required_,

    required this.currentTermsVersion,

    required this.acceptedTermsVersion,

    required this.handle,

    required this.displayName,

    required this.bio,

    required this.profileVisibility,

    required this.activityVisibility,

    required this.discoverable,

    required this.completedAt,
  });

  @JsonKey(name: r'required', required: true, includeIfNull: false)
  final bool required_;

  @JsonKey(name: r'currentTermsVersion', required: true, includeIfNull: false)
  final String currentTermsVersion;

  @JsonKey(name: r'acceptedTermsVersion', required: true, includeIfNull: true)
  final String? acceptedTermsVersion;

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

  @JsonKey(name: r'completedAt', required: true, includeIfNull: true)
  final int? completedAt;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is OnboardingState &&
          other.required_ == required_ &&
          other.currentTermsVersion == currentTermsVersion &&
          other.acceptedTermsVersion == acceptedTermsVersion &&
          other.handle == handle &&
          other.displayName == displayName &&
          other.bio == bio &&
          other.profileVisibility == profileVisibility &&
          other.activityVisibility == activityVisibility &&
          other.discoverable == discoverable &&
          other.completedAt == completedAt;

  @override
  int get hashCode =>
      required_.hashCode +
      currentTermsVersion.hashCode +
      (acceptedTermsVersion == null ? 0 : acceptedTermsVersion.hashCode) +
      handle.hashCode +
      (displayName == null ? 0 : displayName.hashCode) +
      (bio == null ? 0 : bio.hashCode) +
      profileVisibility.hashCode +
      activityVisibility.hashCode +
      discoverable.hashCode +
      (completedAt == null ? 0 : completedAt.hashCode);

  factory OnboardingState.fromJson(Map<String, dynamic> json) =>
      _$OnboardingStateFromJson(json);

  Map<String, dynamic> toJson() => _$OnboardingStateToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
