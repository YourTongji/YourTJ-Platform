import 'package:flutter/material.dart';
import 'package:flutter_markdown_plus/flutter_markdown_plus.dart';
import 'package:markdown/markdown.dart' as markdown;
import 'package:yourtj_api/yourtj_api.dart';

import '../navigation/external_link.dart';

class PlatformContent extends StatelessWidget {
  const PlatformContent({
    required this.source,
    required this.format,
    this.assetDeliveries = const <int, Uri>{},
    this.assetBuilder,
    this.onInternalLink,
    super.key,
  });

  static final RegExp _canonicalAsset = RegExp(r'^yourtj-asset:([1-9][0-9]*)$');
  static final RegExp _dangerousHtmlBlock = RegExp(
    r'<(?:script|iframe|style)\b[^>]*>[\s\S]*?<\/(?:script|iframe|style)\s*>',
    caseSensitive: false,
  );
  static final RegExp _htmlTag = RegExp(
    r'<\/?[A-Za-z][^>]*>',
    caseSensitive: false,
  );

  final String source;
  final ContentFormat format;
  final Map<int, Uri> assetDeliveries;
  final Widget Function(Uri delivery, String? label)? assetBuilder;
  final ValueChanged<String>? onInternalLink;

  @override
  Widget build(BuildContext context) {
    if (format != ContentFormat.markdownV1 || source.length > 50000) {
      return SelectableText(source);
    }
    final String safeSource = source
        .replaceAll(_dangerousHtmlBlock, '')
        .replaceAll(_htmlTag, '');
    return MarkdownBody(
      data: safeSource,
      selectable: true,
      extensionSet: markdown.ExtensionSet.gitHubWeb,
      imageBuilder: (Uri uri, String? title, String? alt) {
        final RegExpMatch? match = _canonicalAsset.firstMatch(uri.toString());
        final int? assetId = match == null
            ? null
            : int.tryParse(match.group(1)!);
        final Uri? delivery = assetId == null ? null : assetDeliveries[assetId];
        if (delivery == null || delivery.scheme != 'https') {
          return _UnavailableImage(label: alt);
        }
        final Widget? resolvedAsset = assetBuilder?.call(delivery, alt);
        if (resolvedAsset != null) {
          return resolvedAsset;
        }
        return Semantics(
          image: true,
          label: alt?.trim().isNotEmpty == true ? alt!.trim() : '正文图片',
          child: ClipRRect(
            borderRadius: BorderRadius.circular(12),
            child: Image.network(
              delivery.toString(),
              fit: BoxFit.contain,
              errorBuilder:
                  (
                    BuildContext context,
                    Object error,
                    StackTrace? stackTrace,
                  ) => _UnavailableImage(label: alt),
            ),
          ),
        );
      },
      onTapLink: (String text, String? href, String title) {
        if (href == null) {
          return;
        }
        final Uri? uri = Uri.tryParse(href);
        if (uri == null) {
          return;
        }
        if (!uri.hasScheme && href.startsWith('/')) {
          onInternalLink?.call(href);
          return;
        }
        if (isAllowedExternalHttps(uri)) {
          confirmAndOpenExternalHttps(context, uri);
        }
      },
    );
  }
}

class _UnavailableImage extends StatelessWidget {
  const _UnavailableImage({required this.label});

  final String? label;

  @override
  Widget build(BuildContext context) {
    final String description = label?.trim().isNotEmpty == true
        ? label!.trim()
        : '未命名图片';
    return Semantics(
      image: true,
      label: '图片当前不可用：$description',
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Theme.of(context).colorScheme.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              const Icon(Icons.broken_image_outlined),
              const SizedBox(width: 8),
              Flexible(child: Text('图片当前不可用：$description')),
            ],
          ),
        ),
      ),
    );
  }
}
