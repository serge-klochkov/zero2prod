use sqlx::{Postgres, Transaction};

pub type Tx<'a> = Transaction<'a, Postgres>;
