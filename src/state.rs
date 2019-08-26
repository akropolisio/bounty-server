use once_cell::sync::OnceCell;

static STATE: OnceCell<State> = OnceCell::INIT;

pub struct State {
    pool: crate::db::TheConnectionPool,
}

impl State {
    pub fn get() -> &'static State {
        STATE.get().expect("The State is not initialized")
    }

    pub fn initialize(state: State) {
        if STATE.set(state).is_err() {
            panic!("Cant init State");
        }
    }

    pub fn new(pool: crate::db::TheConnectionPool) -> Self {
        Self { pool }
    }

    pub fn get_pool(&self) -> crate::db::TheConnectionPool {
        std::sync::Arc::clone(&self.pool)
    }
}
