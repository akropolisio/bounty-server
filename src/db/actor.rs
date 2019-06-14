//! Diesel actor for Actix

use std::sync::Arc;
use diesel::prelude::*;
use diesel::r2d2;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use diesel::query_builder::{SelectQuery, QueryFragment};
use ::actix::prelude::*;
use diesel::query_dsl::LoadQuery;
use std::marker::PhantomData;
use std::env;
use dotenv::dotenv;

use log::debug;


#[cfg(feature = "dbpool")]
pub type TheConnectionPool = Arc<r2d2::Pool<diesel::r2d2::ConnectionManager<TheConnection>>>;

#[cfg(feature = "sqlite")]
mod support {
	pub type TheConnection = diesel::SqliteConnection;
	pub type TheBackend = diesel::sqlite::Sqlite;
}

#[cfg(feature = "postgres")]
mod support {
	pub type TheConnection = diesel::PgConnection;
	pub type TheBackend = diesel::pg::Pg;
}

pub use support::*;


pub struct DbExecutor<T: 'static>
	where T: Connection {
	pub pool: Pool<ConnectionManager<T>>,
}

impl<T: Connection> Actor for DbExecutor<T> {
	type Context = SyncContext<Self>;

	fn started(&mut self, _ctx: &mut Self::Context) {
		debug!("I am alive!");
	}
}

pub struct ReadQuery<S, I> {
	pub select: S,
	pub phantom: PhantomData<I>,
}

impl<S, I: 'static> Message for ReadQuery<S, I> {
	type Result = Result<Vec<I>, Error>;
}

impl<S: LoadQuery<TheConnection, I> + SelectQuery + QueryFragment<TheBackend>, I: 'static> Handler<ReadQuery<S, I>>
	for DbExecutor<TheConnection>
{
	type Result = Result<Vec<I>, Error>;

	fn handle(&mut self, msg: ReadQuery<S, I>, _: &mut Self::Context) -> Self::Result {
		let pool = &self.pool;
		let dbg = diesel::debug_query(&msg.select);
		debug!("{:?}", dbg);
		if let Ok(conn) = pool.get() {
			let res = msg.select.load::<I>(&conn);
			return res;
		}
		Ok(Vec::new())
	}
}

pub struct WriteQuery<W, I> {
	pub query: W,
	pub phantom: PhantomData<I>,
}

impl<W, I: 'static> Message for WriteQuery<W, I> {
	type Result = Result<Vec<I>, Error>;
}

impl<W: LoadQuery<TheConnection, I> + QueryFragment<TheBackend>, I: 'static> Handler<WriteQuery<W, I>>
	for DbExecutor<TheConnection>
{
	type Result = Result<Vec<I>, Error>;

	fn handle(&mut self, msg: WriteQuery<W, I>, _: &mut Self::Context) -> Self::Result {
		let pool = &self.pool;
		let dbg = diesel::debug_query(&msg.query);
		debug!("{:?}", dbg);
		if let Ok(conn) = pool.get() {
			// let res = msg.query.execute(&conn);
			let res = msg.query.get_results::<I>(&conn);
			return res;
		}
		Ok(Vec::new())
	}
}


pub struct State {
	pub rdb: Addr<DbExecutor<TheConnection>>,
	pub wdb: Addr<DbExecutor<TheConnection>>,
}

/// Use the Read setting with a connection String to access a read-only db replica
/// Use the Write setting to udpate with a connection String to a writeable DB
pub enum ConnectionType {
	Read,
	Write,
}


pub fn db_init() -> State {
	let raddr = db_setup(ConnectionType::Read);
	let waddr = db_setup(ConnectionType::Write);

	State { rdb: raddr, wdb: waddr }
}

pub fn db_setup(conn_type: ConnectionType) -> actix::Addr<DbExecutor<TheConnection>> {
	dotenv().ok();

	let var = match conn_type {
		ConnectionType::Read => "DB_READ_URL",
		ConnectionType::Write => "DB_WRITE_URL",
	};
	let database_url = env::var(var).unwrap_or_else(|_| panic!("{} must be set", var));
	let manager = ConnectionManager::<TheConnection>::new(database_url);
	let pool = r2d2::Pool::builder().build(manager).expect("Failed to create pool.");
	SyncArbiter::start(3, move || DbExecutor { pool: pool.clone() })
}

#[derive(Debug)]
pub enum DbExecutorError {
	DatabaseError(diesel::result::Error),
	MailBoxError(actix::MailboxError),
	Unknown,
}

impl From<diesel::result::Error> for DbExecutorError {
	fn from(error: diesel::result::Error) -> Self { DbExecutorError::DatabaseError(error) }
}

impl From<actix::MailboxError> for DbExecutorError {
	fn from(error: actix::MailboxError) -> Self { DbExecutorError::MailBoxError(error) }
}
