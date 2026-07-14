// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'thread_pin_input.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ThreadPinInput _$ThreadPinInputFromJson(Map<String, dynamic> json) =>
    $checkedCreate('ThreadPinInput', json, ($checkedConvert) {
      final val = ThreadPinInput(
        globally: $checkedConvert('globally', (v) => v as bool?),
      );
      return val;
    });

Map<String, dynamic> _$ThreadPinInputToJson(ThreadPinInput instance) =>
    <String, dynamic>{'globally': ?instance.globally};
