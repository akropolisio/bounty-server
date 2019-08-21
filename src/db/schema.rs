table! {
    logs (id) {
        id -> Int8,
        token -> Text,
        action -> Text,
        payload -> Jsonb,
        created_at -> Timestamptz,
    }
}

table! {
    tokens (token) {
        token -> Text,
        created_at -> Timestamptz,
        expired_at -> Timestamptz,
    }
}

table! {
    users (id) {
        id -> Int4,
        terms_signed -> Bool,
        not_resident -> Bool,
        address -> Varchar,
        amount -> Int8,
    }
}

allow_tables_to_appear_in_same_query!(logs, tokens, users,);
