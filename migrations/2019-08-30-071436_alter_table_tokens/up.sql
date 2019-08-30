ALTER TABLE tokens ALTER expired_at SET DEFAULT now() + interval '600 seconds';
