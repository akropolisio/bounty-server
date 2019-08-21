CREATE TABLE tokens (
    token TEXT NOT NULL PRIMARY KEY,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    expired_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now() + interval '300 seconds'
);

CREATE INDEX tokens_token_idx on tokens(token);
CREATE INDEX tokens_expired_at_idx on tokens(expired_at);
