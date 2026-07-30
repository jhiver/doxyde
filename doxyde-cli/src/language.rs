use anyhow::{anyhow, Context, Result};
use sqlx::SqlitePool;

fn validate_language_code(code: &str) -> Result<&str> {
    if doxyde_web::languages::is_known(code) {
        return Ok(code);
    }

    Err(anyhow!("Unknown language code: {code}"))
}

pub(crate) async fn enable_language(pool: &SqlitePool, code: &str) -> Result<bool> {
    let code = validate_language_code(code)?;
    let result = sqlx::query(
        "INSERT OR IGNORE INTO i18n_enabled_lang (lang, position)
         SELECT ?1, COALESCE(MAX(position), -1) + 1
         FROM i18n_enabled_lang",
    )
    .bind(code)
    .execute(pool)
    .await
    .context("Failed to enable language")?;

    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[test]
    fn known_language_code_is_valid() -> Result<()> {
        assert_eq!(validate_language_code("de")?, "de");
        Ok(())
    }

    #[test]
    fn unknown_language_code_is_rejected() {
        assert!(validate_language_code("xx").is_err());
    }

    #[tokio::test]
    async fn enabling_language_appends_it_once() -> Result<()> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        sqlx::query("CREATE TABLE i18n_enabled_lang (lang TEXT PRIMARY KEY, position INTEGER)")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO i18n_enabled_lang VALUES ('en', 0), ('fr', 5)")
            .execute(&pool)
            .await?;
        assert!(enable_language(&pool, "de").await?);
        assert!(!enable_language(&pool, "de").await?);
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM i18n_enabled_lang")
            .fetch_one(&pool)
            .await?;
        let position: i64 =
            sqlx::query_scalar("SELECT position FROM i18n_enabled_lang WHERE lang = 'de'")
                .fetch_one(&pool)
                .await?;
        assert_eq!(count, 3);
        assert_eq!(position, 6);
        Ok(())
    }
}
