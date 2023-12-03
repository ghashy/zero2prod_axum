--! insert_new_subscription
INSERT INTO subscriptions(id, email, name, subscribed_at, status)
VALUES (:id, :email, :name, :subscribed_at, 'pending_confirmation');

--! insert_new_token
INSERT INTO subscription_tokens(subscription_token, subscriber_id)
VALUES (:subscription_token, :subscriber_id);

--! delete_token_by_email
DELETE FROM subscription_tokens
    WHERE subscriber_id = (
    SELECT id
    FROM subscriptions
    WHERE email = :email
);

--! insert_token_by_email
INSERT INTO subscription_tokens
(subscription_token, subscriber_id)
VALUES (:token, (
        SELECT id
        FROM subscriptions
        WHERE email = :email
    )
);

--! get_status
SELECT status
FROM subscriptions
WHERE email = :email;

--! confirm_subscriber
UPDATE subscriptions SET status = 'confirmed' WHERE id = :sub_id AND status = 'pending_confirmation';

--! get_subscriber_id_from_token
SELECT subscriber_id FROM subscription_tokens WHERE subscription_token = :sub_token;

--! get_countdown
WITH RECURSIVE countdown(val) AS (
    SELECT 10 AS val -- initial, non-recursive query
    UNION -- every recursive CTE needs `UNION` keyword
    SELECT val - 1 FROM countdown WHERE val > 1 -- recursive query
)
SELECT *
FROM countdown;

