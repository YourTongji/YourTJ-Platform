ALTER TABLE forum.dm_messages
  ADD COLUMN client_message_id UUID;

CREATE UNIQUE INDEX dm_messages_sender_client_message_unique
  ON forum.dm_messages (sender_id, client_message_id)
  WHERE client_message_id IS NOT NULL;
