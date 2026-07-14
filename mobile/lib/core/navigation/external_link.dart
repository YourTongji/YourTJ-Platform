import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

bool isAllowedExternalHttps(Uri uri) {
  return uri.isAbsolute &&
      uri.scheme == 'https' &&
      uri.host.isNotEmpty &&
      uri.userInfo.isEmpty;
}

Future<bool> confirmAndOpenExternalHttps(BuildContext context, Uri uri) async {
  if (!isAllowedExternalHttps(uri)) {
    return false;
  }
  final bool? shouldOpen = await showDialog<bool>(
    context: context,
    builder: (BuildContext dialogContext) {
      return AlertDialog(
        title: const Text('打开外部链接？'),
        content: Text('即将离开 YourTJ 并访问 ${uri.host}。'),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            child: const Text('继续'),
          ),
        ],
      );
    },
  );
  if (shouldOpen != true || !context.mounted) {
    return false;
  }
  try {
    final bool opened = await launchUrl(
      uri,
      mode: LaunchMode.externalApplication,
    );
    if (!opened && context.mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('无法打开该 HTTPS 链接')));
    }
    return opened;
  } on Object {
    if (context.mounted) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(const SnackBar(content: Text('无法打开该 HTTPS 链接')));
    }
    return false;
  }
}
