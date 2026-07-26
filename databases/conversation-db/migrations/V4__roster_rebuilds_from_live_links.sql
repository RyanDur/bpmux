-- roster is a projection of live links; the store holds no roster snapshots
DELETE FROM messages
WHERE message_type = 'state-delta'
  AND payload LIKE '%"domain":"roster"%';
