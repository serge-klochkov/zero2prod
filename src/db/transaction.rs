use anyhow::Context;
use sqlx::PgPool;

use crate::db::types::Tx;

pub async fn begin_transaction(pg_pool: &PgPool) -> anyhow::Result<Tx<'_>> {
    let tx = pg_pool
        .begin()
        .await
        .context("Failed to acquire a transaction")?;
    Ok(tx)
}

pub async fn commit_transaction(tx: Tx<'_>) -> anyhow::Result<()> {
    tx.commit()
        .await
        .context("Failed to commit the transaction")?;
    Ok(())
}
