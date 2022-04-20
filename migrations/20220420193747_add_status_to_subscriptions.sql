BEGIN;
    CREATE TYPE subscription_status AS ENUM ('pending', 'confirmed', 'failed');

    ALTER TABLE subscriptions
    ADD COLUMN status subscription_status NULL;

    UPDATE subscriptions
    SET status = 'confirmed'
    WHERE status IS NULL;

    ALTER TABLE subscriptions
    ALTER COLUMN status SET NOT NULL;
COMMIT;
