import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:yourtj_api/yourtj_api.dart';

import '../../../app/app_services.dart';
import '../data/media_uploader.dart';

class MediaUploadButton extends ConsumerStatefulWidget {
  const MediaUploadButton({
    required this.kind,
    required this.onUploaded,
    this.usage,
    this.label,
    super.key,
  });

  final MediaUploadKind kind;
  final MediaUsage? usage;
  final ValueChanged<CompletedMediaUpload> onUploaded;
  final String? label;

  @override
  ConsumerState<MediaUploadButton> createState() => _MediaUploadButtonState();
}

class _MediaUploadButtonState extends ConsumerState<MediaUploadButton> {
  bool _isUploading = false;
  double? _progress;

  @override
  Widget build(BuildContext context) {
    final String label =
        widget.label ??
        (widget.kind == MediaUploadKind.image ? '上传图片' : '上传 PDF');
    return Semantics(
      liveRegion: _isUploading,
      label: _isUploading && _progress != null
          ? '$label，已上传 ${(_progress! * 100).round()}%'
          : label,
      child: OutlinedButton.icon(
        onPressed: _isUploading ? null : _selectAndUpload,
        icon: _isUploading
            ? const SizedBox.square(
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2),
              )
            : const Icon(Icons.upload_file_outlined),
        label: Text(
          _isUploading && _progress != null
              ? '${(_progress! * 100).round()}%'
              : label,
        ),
      ),
    );
  }

  Future<void> _selectAndUpload() async {
    final MediaUploader uploader = ref.read(mediaUploaderProvider);
    late final MediaUploadOwner owner;
    try {
      owner = uploader.captureOwner();
    } on MediaUploadFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
      return;
    }
    final XFile? file = await openFile(
      acceptedTypeGroups: <XTypeGroup>[
        widget.kind == MediaUploadKind.image ? _staticImages : _pdfFiles,
      ],
      confirmButtonText: '选择',
    );
    if (file == null || !mounted) {
      return;
    }
    try {
      uploader.ensureCurrentOwner(owner);
    } on MediaUploadFailure catch (failure) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(failure.message)));
      return;
    }
    setState(() {
      _isUploading = true;
      _progress = null;
    });
    try {
      final CompletedMediaUpload completed = await uploader.upload(
        owner: owner,
        file: file,
        kind: widget.kind,
        usage: widget.usage,
        onProgress: (int sent, int total) {
          if (mounted && total > 0) {
            setState(() => _progress = sent / total);
          }
        },
      );
      if (mounted) {
        uploader.ensureCurrentOwner(owner);
        widget.onUploaded(completed);
      }
    } on MediaUploadFailure catch (failure) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text(failure.message)));
      }
    } finally {
      if (mounted) {
        setState(() {
          _isUploading = false;
          _progress = null;
        });
      }
    }
  }
}

const XTypeGroup _staticImages = XTypeGroup(
  label: '静态图片',
  extensions: <String>['jpg', 'jpeg', 'png', 'webp'],
  mimeTypes: <String>['image/jpeg', 'image/png', 'image/webp'],
  uniformTypeIdentifiers: <String>[
    'public.jpeg',
    'public.png',
    'org.webmproject.webp',
  ],
  webWildCards: <String>['image/jpeg', 'image/png', 'image/webp'],
);

const XTypeGroup _pdfFiles = XTypeGroup(
  label: 'PDF',
  extensions: <String>['pdf'],
  mimeTypes: <String>['application/pdf'],
  uniformTypeIdentifiers: <String>['com.adobe.pdf'],
  webWildCards: <String>['application/pdf'],
);
