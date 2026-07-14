// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'ignore_user.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

IgnoreUser _$IgnoreUserFromJson(Map<String, dynamic> json) =>
    $checkedCreate('IgnoreUser', json, ($checkedConvert) {
      final val = IgnoreUser(
        accountId: $checkedConvert('accountId', (v) => v as String?),
        handle: $checkedConvert('handle', (v) => v as String?),
        avatarUrl: $checkedConvert('avatarUrl', (v) => v as String?),
        createdAt: $checkedConvert('createdAt', (v) => (v as num?)?.toInt()),
      );
      return val;
    });

Map<String, dynamic> _$IgnoreUserToJson(IgnoreUser instance) =>
    <String, dynamic>{
      'accountId': ?instance.accountId,
      'handle': ?instance.handle,
      'avatarUrl': ?instance.avatarUrl,
      'createdAt': ?instance.createdAt,
    };
