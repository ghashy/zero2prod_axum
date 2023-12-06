--! query_confirmed_subscribers
SELECT email
FROM subscriptions
WHERE status = 'confirmed';
