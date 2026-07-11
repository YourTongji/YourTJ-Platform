UPDATE forum.boards AS board
SET thread_count = (
    SELECT COUNT(*)::INTEGER
    FROM forum.threads AS thread
    WHERE thread.board_id = board.id
      AND thread.status = 'visible'
      AND thread.deleted_at IS NULL
      AND thread.hidden_at IS NULL
      AND thread.archived_at IS NULL
);
