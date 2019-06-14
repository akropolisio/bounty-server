#[cfg(all(feature = "sqlite", feature = "postgres"))]
compile_error!("Features 'sqlite' and 'postgres' is mutually exclusive. Select one of their.");
#[cfg(all(not(feature = "sqlite"), not(feature = "postgres")))]
compile_error!("Required one of the folowing features: 'sqlite' and 'postgres'.");


use diesel::prelude::*;
// use dotenv::dotenv;

// use std::env;
use std::sync::Arc;

// pub mod actor;
pub mod schema;
pub mod models;


#[cfg(feature = "sqlite")]
pub type TheConnection = diesel::SqliteConnection;
#[cfg(feature = "postgres")]
pub type TheConnection = diesel::PgConnection;
#[cfg(feature = "dbpool")]
pub type TheConnectionPool = Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<TheConnection>>>;


pub fn establish_connection(database_url: &str) -> TheConnection {
	TheConnection::establish(&database_url).unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[cfg(feature = "dbpool")]
pub fn establish_connection_pool(max_size: u32, database_url: &str) -> TheConnectionPool {
	use diesel::r2d2::Pool;
	use diesel::r2d2::ConnectionManager;

	let manager = ConnectionManager::<TheConnection>::new(database_url);
	let pool = Arc::new(Pool::builder().max_size(max_size).build(manager).unwrap());
	pool
}
