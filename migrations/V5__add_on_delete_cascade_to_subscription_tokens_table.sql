ALTER TABLE subscription_tokens DROP CONSTRAINT subscription_tokens_subscriber_id_fkey;
ALTER TABLE subscription_tokens ADD FOREIGN KEY (subscriber_id) REFERENCES subscriptions(id) ON DELETE CASCADE;

