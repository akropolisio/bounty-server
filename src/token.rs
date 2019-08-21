use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, Header};
use uuid::Uuid;

use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng, distributions::Alphanumeric};

lazy_static! {
    static ref SECRET: String = thread_rng().sample_iter(&Alphanumeric).take(20).collect();
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    timestamp: u128,
    uid: String,
}

pub fn create_token() -> String {
    let my_claims = Claims { timestamp: current_timestamp(), uid: Uuid::new_v4().to_string() };
    encode(&Header::default(), &my_claims, SECRET.as_ref())
        .expect("Can not create new token")
}

fn current_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Can not get current timestamp")
        .as_nanos()
}
