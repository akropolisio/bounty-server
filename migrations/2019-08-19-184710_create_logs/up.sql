CREATE TABLE logs (
    id BIGSERIAL PRIMARY KEY,
    token TEXT NOT NULL,
    action TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

CREATE INDEX logs_token_action_idx on logs(token, action);
CREATE INDEX logs_action_idx on logs(action);
