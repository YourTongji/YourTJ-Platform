//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:yourtj_api/src/model/account_lifecycle.dart';
import 'package:yourtj_api/src/model/recovery_credential.dart';
import 'package:json_annotation/json_annotation.dart';

part 'account_lifecycle_mutation.g.dart';

@JsonSerializable(
  checked: true,
  createToJson: true,
  disallowUnrecognizedKeys: false,
  explicitToJson: true,
)
class AccountLifecycleMutation {
  /// Returns a new [AccountLifecycleMutation] instance.
  AccountLifecycleMutation({required this.lifecycle, required this.recovery});

  @JsonKey(name: r'lifecycle', required: true, includeIfNull: false)
  final AccountLifecycle lifecycle;

  @JsonKey(name: r'recovery', required: true, includeIfNull: false)
  final RecoveryCredential recovery;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is AccountLifecycleMutation &&
          other.lifecycle == lifecycle &&
          other.recovery == recovery;

  @override
  int get hashCode => lifecycle.hashCode + recovery.hashCode;

  factory AccountLifecycleMutation.fromJson(Map<String, dynamic> json) =>
      _$AccountLifecycleMutationFromJson(json);

  Map<String, dynamic> toJson() => _$AccountLifecycleMutationToJson(this);

  @override
  String toString() {
    return toJson().toString();
  }
}
