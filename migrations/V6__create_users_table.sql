CREATE TABLE users(
   user_id uuid PRIMARY KEY,
   username VARCHAR(64) NOT NULL UNIQUE,
   password VARCHAR(64) NOT NULL
);
