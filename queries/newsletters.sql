--! query_confirmed_subscribers
SELECT email
FROM subscriptions
WHERE status = 'confirmed';

--! query_user_id_by_credentials
SELECT user_id
FROM users
WHERE username = :name AND password_hash = :pass;
