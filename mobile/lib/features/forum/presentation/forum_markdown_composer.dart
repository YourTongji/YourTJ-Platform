import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

import 'forum_widgets.dart';

enum _ComposerView { edit, preview }

/// Keeps forum Markdown editing and preview on the same renderer profile.
class ForumMarkdownComposer extends StatefulWidget {
  const ForumMarkdownComposer({
    required this.controller,
    required this.label,
    required this.maxLength,
    required this.minLines,
    required this.maxLines,
    this.helperText,
    this.attachments = const <ForumAttachment>[],
    this.onRefreshDelivery,
    super.key,
  });

  final TextEditingController controller;
  final String label;
  final int maxLength;
  final int minLines;
  final int maxLines;
  final String? helperText;
  final List<ForumAttachment> attachments;
  final VoidCallback? onRefreshDelivery;

  @override
  State<ForumMarkdownComposer> createState() => _ForumMarkdownComposerState();
}

class _ForumMarkdownComposerState extends State<ForumMarkdownComposer> {
  _ComposerView _view = _ComposerView.edit;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_handleSourceChanged);
  }

  @override
  void didUpdateWidget(ForumMarkdownComposer oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_handleSourceChanged);
      widget.controller.addListener(_handleSourceChanged);
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_handleSourceChanged);
    super.dispose();
  }

  void _handleSourceChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) {
    final String source = widget.controller.text;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: <Widget>[
        Semantics(
          container: true,
          label: '${widget.label}显示方式',
          child: SegmentedButton<_ComposerView>(
            segments: const <ButtonSegment<_ComposerView>>[
              ButtonSegment<_ComposerView>(
                value: _ComposerView.edit,
                icon: Icon(Icons.edit_outlined),
                label: Text('编辑'),
              ),
              ButtonSegment<_ComposerView>(
                value: _ComposerView.preview,
                icon: Icon(Icons.visibility_outlined),
                label: Text('预览'),
              ),
            ],
            selected: <_ComposerView>{_view},
            onSelectionChanged: (Set<_ComposerView> selected) {
              setState(() => _view = selected.single);
            },
          ),
        ),
        const SizedBox(height: 10),
        if (_view == _ComposerView.edit)
          TextField(
            controller: widget.controller,
            minLines: widget.minLines,
            maxLines: widget.maxLines,
            maxLength: widget.maxLength,
            decoration: InputDecoration(
              labelText: widget.label,
              alignLabelWithHint: true,
              helperText: widget.helperText,
            ),
          )
        else
          Semantics(
            container: true,
            label: '${widget.label} Markdown 预览',
            child: ConstrainedBox(
              constraints: const BoxConstraints(minHeight: 160),
              child: DecoratedBox(
                decoration: BoxDecoration(
                  border: Border.all(
                    color: Theme.of(context).colorScheme.outlineVariant,
                  ),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(16),
                  child: source.trim().isEmpty
                      ? Center(
                          child: Text(
                            '没有可预览的内容',
                            style: Theme.of(context).textTheme.bodyMedium,
                          ),
                        )
                      : ForumBody(
                          source: source,
                          format: ContentFormat.markdownV1,
                          attachments: widget.attachments,
                          onRefreshDelivery: widget.onRefreshDelivery,
                        ),
                ),
              ),
            ),
          ),
        const SizedBox(height: 6),
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                '支持 CommonMark 与 GFM；不解析 HTML 或远程图片',
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
            const SizedBox(width: 12),
            Text(
              '${source.length}/${widget.maxLength}',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ),
      ],
    );
  }
}
