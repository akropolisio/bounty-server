table! {
	users (id) {
		id -> Integer,
		terms_signed -> Bool,
		not_resident -> Bool,
		address -> Text,
		amount -> BigInt,
	}
}
