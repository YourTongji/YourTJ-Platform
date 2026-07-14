import 'package:flutter/material.dart';
import 'package:yourtj_api/yourtj_api.dart';

Future<AnnouncementReceiptInputActionEnum?> showAnnouncementDialog({
  required BuildContext context,
  required Announcement announcement,
  VoidCallback? onPresented,
}) {
  return showDialog<AnnouncementReceiptInputActionEnum>(
    context: context,
    barrierDismissible: !announcement.requiresAck,
    builder: (BuildContext context) => AnnouncementDialog(
      announcement: announcement,
      onPresented: onPresented,
    ),
  );
}

class AnnouncementDialog extends StatefulWidget {
  const AnnouncementDialog({
    required this.announcement,
    this.onPresented,
    super.key,
  });

  final Announcement announcement;
  final VoidCallback? onPresented;

  @override
  State<AnnouncementDialog> createState() => _AnnouncementDialogState();
}

class _AnnouncementDialogState extends State<AnnouncementDialog> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((Duration _) {
      if (mounted) {
        widget.onPresented?.call();
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final ({String label, IconData icon}) severity = _severity(
      widget.announcement.severity,
    );
    return AlertDialog(
      icon: Icon(severity.icon),
      title: Text(widget.announcement.title),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560, maxHeight: 520),
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('${severity.label} · 公告版本 ${widget.announcement.revision}'),
              if (widget.announcement.requiresAck)
                const Padding(
                  padding: EdgeInsets.only(top: 6),
                  child: Text('本公告需要明确确认；选择稍后处理不会记为已确认。'),
                ),
              if (widget.announcement.body case final String body) ...<Widget>[
                const SizedBox(height: 16),
                Text(body),
              ],
              const SizedBox(height: 16),
              Text('有效期：${announcementScheduleText(widget.announcement)}'),
            ],
          ),
        ),
      ),
      actions: <Widget>[
        if (widget.announcement.requiresAck)
          TextButton(
            onPressed: () => Navigator.of(
              context,
            ).pop(AnnouncementReceiptInputActionEnum.dismiss),
            child: const Text('稍后处理'),
          ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(
            widget.announcement.requiresAck
                ? AnnouncementReceiptInputActionEnum.acknowledge
                : AnnouncementReceiptInputActionEnum.dismiss,
          ),
          child: Text(widget.announcement.requiresAck ? '我已知晓' : '知道了'),
        ),
      ],
    );
  }
}

String announcementScheduleText(Announcement announcement) {
  final String starts = announcement.startsAt == null
      ? '立即生效'
      : _formatUnix(announcement.startsAt!);
  final String ends = announcement.endsAt == null
      ? '长期有效'
      : _formatUnix(announcement.endsAt!);
  return '$starts — $ends';
}

({String label, IconData icon}) _severity(AnnouncementSeverityEnum severity) {
  return switch (severity) {
    AnnouncementSeverityEnum.info => (
      label: '平台信息',
      icon: Icons.info_outline_rounded,
    ),
    AnnouncementSeverityEnum.success => (
      label: '平台进展',
      icon: Icons.check_circle_outline_rounded,
    ),
    AnnouncementSeverityEnum.warning => (
      label: '重要提醒',
      icon: Icons.warning_amber_rounded,
    ),
    AnnouncementSeverityEnum.critical => (
      label: '紧急公告',
      icon: Icons.shield_outlined,
    ),
    AnnouncementSeverityEnum.unknownDefaultOpenApi => (
      label: '公告',
      icon: Icons.campaign_outlined,
    ),
  };
}

String _formatUnix(int seconds) {
  final DateTime value = DateTime.fromMillisecondsSinceEpoch(
    seconds * 1000,
    isUtc: true,
  ).toLocal();
  String two(int number) => number.toString().padLeft(2, '0');
  return '${value.year}-${two(value.month)}-${two(value.day)} '
      '${two(value.hour)}:${two(value.minute)}';
}
