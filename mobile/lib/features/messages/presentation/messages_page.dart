import 'package:flutter/material.dart';

import '../../announcements/presentation/announcements_page.dart';
import '../../notifications/presentation/notifications_page.dart';
import '../data/messages_repository.dart';
import 'direct_messages_page.dart';

enum MessageCenterSection { notifications, directMessages, announcements }

class MessagesPage extends StatefulWidget {
  const MessagesPage({
    this.initialSection = MessageCenterSection.notifications,
    this.initialConversationId,
    this.initialView = ConversationView.inbox,
    super.key,
  });

  final MessageCenterSection initialSection;
  final String? initialConversationId;
  final ConversationView initialView;

  @override
  State<MessagesPage> createState() => _MessagesPageState();
}

class _MessagesPageState extends State<MessagesPage> {
  late MessageCenterSection _section = widget.initialSection;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('消息中心')),
      body: SafeArea(
        top: false,
        child: Column(
          children: <Widget>[
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
              child: SegmentedButton<MessageCenterSection>(
                segments: const <ButtonSegment<MessageCenterSection>>[
                  ButtonSegment<MessageCenterSection>(
                    value: MessageCenterSection.notifications,
                    icon: Icon(Icons.notifications_outlined),
                    label: Text('通知'),
                  ),
                  ButtonSegment<MessageCenterSection>(
                    value: MessageCenterSection.directMessages,
                    icon: Icon(Icons.forum_outlined),
                    label: Text('私信'),
                  ),
                  ButtonSegment<MessageCenterSection>(
                    value: MessageCenterSection.announcements,
                    icon: Icon(Icons.campaign_outlined),
                    label: Text('公告'),
                  ),
                ],
                selected: <MessageCenterSection>{_section},
                onSelectionChanged: (Set<MessageCenterSection> selection) {
                  setState(() => _section = selection.single);
                },
              ),
            ),
            Expanded(
              child: IndexedStack(
                index: _section.index,
                children: <Widget>[
                  const NotificationsPage(embedded: true),
                  DirectMessagesPage(
                    initialConversationId: widget.initialConversationId,
                    initialView: widget.initialView,
                  ),
                  const AnnouncementsPage(embedded: true),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
