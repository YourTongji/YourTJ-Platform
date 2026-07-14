import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../config/app_environment.dart';

/// Makes the current environment's platform-media origins available to image widgets.
class PlatformMediaScope extends InheritedWidget {
  const PlatformMediaScope({
    required this.environment,
    required super.child,
    super.key,
  });

  final AppEnvironment environment;

  static AppEnvironment? maybeEnvironmentOf(BuildContext context) {
    return context
        .dependOnInheritedWidgetOfExactType<PlatformMediaScope>()
        ?.environment;
  }

  @override
  bool updateShouldNotify(PlatformMediaScope oldWidget) {
    return oldWidget.environment.apiBaseUri != environment.apiBaseUri ||
        oldWidget.environment.mediaCdnBaseUri != environment.mediaCdnBaseUri;
  }
}

/// Renders a platform avatar without trusting stale or non-HTTPS delivery URLs.
class PlatformAvatar extends StatefulWidget {
  const PlatformAvatar({
    required this.fallbackText,
    this.delivery,
    this.compatibilityUrl,
    this.onRefresh,
    this.radius = 20,
    this.semanticLabel,
    super.key,
  });

  final String fallbackText;
  final MediaDelivery? delivery;
  final String? compatibilityUrl;
  final VoidCallback? onRefresh;
  final double radius;
  final String? semanticLabel;

  @override
  State<PlatformAvatar> createState() => _PlatformAvatarState();
}

class _PlatformAvatarState extends State<PlatformAvatar> {
  static const int _refreshSkewSeconds = 30;

  bool _hasRequestedRefresh = false;

  @override
  void didUpdateWidget(PlatformAvatar oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_sourceIdentity(oldWidget) != _sourceIdentity(widget)) {
      _hasRequestedRefresh = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final String? url = _deliveryUrl(context);
    final String trimmedFallback = widget.fallbackText.trim();
    final String initial = trimmedFallback.characters.isEmpty
        ? '?'
        : trimmedFallback.characters.first.toUpperCase();
    final String label = widget.semanticLabel?.trim().isNotEmpty == true
        ? widget.semanticLabel!.trim()
        : trimmedFallback.isEmpty
        ? '用户头像'
        : '$trimmedFallback 的头像';

    return Semantics(
      image: true,
      label: label,
      child: CircleAvatar(
        radius: widget.radius,
        backgroundColor: Theme.of(context).colorScheme.primaryContainer,
        foregroundImage: url == null ? null : NetworkImage(url),
        onForegroundImageError: url == null
            ? null
            : (Object error, StackTrace? stackTrace) => _requestRefreshOnce(),
        child: Text(initial),
      ),
    );
  }

  String? _deliveryUrl(BuildContext context) {
    final AppEnvironment? environment = PlatformMediaScope.maybeEnvironmentOf(
      context,
    );
    final MediaDelivery? delivery = widget.delivery;
    if (delivery != null) {
      final int refreshAt = DateTime.now().millisecondsSinceEpoch ~/ 1000;
      final Uri? uri = safePlatformImageUri(delivery.url, environment);
      if (uri == null ||
          delivery.expiresAt <= refreshAt + _refreshSkewSeconds) {
        _requestRefreshOnce();
        return null;
      }
      return uri.toString();
    }

    final String? compatibilityUrl = widget.compatibilityUrl;
    if (compatibilityUrl == null || compatibilityUrl.trim().isEmpty) {
      return null;
    }
    final Uri? uri = safePlatformImageUri(compatibilityUrl, environment);
    if (uri == null) {
      _requestRefreshOnce();
      return null;
    }
    return uri.toString();
  }

  void _requestRefreshOnce() {
    if (_hasRequestedRefresh || widget.onRefresh == null) {
      return;
    }
    _hasRequestedRefresh = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted) {
        widget.onRefresh?.call();
      }
    });
  }
}

/// Accepts only an absolute HTTPS URL on the configured API/CDN origin allowlist.
Uri? safePlatformImageUri(String value, AppEnvironment? environment) {
  final Uri? uri = Uri.tryParse(value.trim());
  if (uri == null ||
      environment == null ||
      uri.scheme != 'https' ||
      !uri.hasAuthority ||
      uri.host.isEmpty ||
      uri.userInfo.isNotEmpty ||
      uri.hasFragment ||
      !environment.ownsPlatformMedia(uri)) {
    return null;
  }
  return uri;
}

/// Renders a legacy URL-backed platform image with fail-closed refresh behavior.
class PlatformImage extends StatefulWidget {
  const PlatformImage({
    required this.url,
    required this.height,
    required this.semanticLabel,
    this.onRefresh,
    this.fit = BoxFit.cover,
    super.key,
  });

  final String url;
  final double height;
  final String semanticLabel;
  final VoidCallback? onRefresh;
  final BoxFit fit;

  @override
  State<PlatformImage> createState() => _PlatformImageState();
}

class _PlatformImageState extends State<PlatformImage> {
  bool _hasRequestedRefresh = false;

  @override
  void didUpdateWidget(PlatformImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url) {
      _hasRequestedRefresh = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final Uri? uri = safePlatformImageUri(
      widget.url,
      PlatformMediaScope.maybeEnvironmentOf(context),
    );
    if (uri == null) {
      _requestRefreshOnce();
      return _fallback(context);
    }
    return Image.network(
      uri.toString(),
      height: widget.height,
      width: double.infinity,
      fit: widget.fit,
      semanticLabel: widget.semanticLabel,
      errorBuilder:
          (BuildContext context, Object error, StackTrace? stackTrace) {
            _requestRefreshOnce();
            return _fallback(context);
          },
    );
  }

  Widget _fallback(BuildContext context) {
    return Semantics(
      image: true,
      label: '${widget.semanticLabel}当前不可用',
      child: SizedBox(
        height: widget.height,
        child: ColoredBox(
          color: Theme.of(context).colorScheme.surfaceContainerHigh,
          child: const Center(child: Icon(Icons.broken_image_outlined)),
        ),
      ),
    );
  }

  void _requestRefreshOnce() {
    if (_hasRequestedRefresh || widget.onRefresh == null) {
      return;
    }
    _hasRequestedRefresh = true;
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted) {
        widget.onRefresh?.call();
      }
    });
  }
}

String _sourceIdentity(PlatformAvatar widget) {
  final MediaDelivery? delivery = widget.delivery;
  if (delivery != null) {
    return '${delivery.assetId}\u0000${delivery.url}\u0000${delivery.expiresAt}';
  }
  return widget.compatibilityUrl ?? '';
}
