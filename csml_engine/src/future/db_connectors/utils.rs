#[cfg(feature = "postgresql-async")]
pub fn get_expires_at_for_postgresql(
    ttl: Option<chrono::Duration>,
) -> Option<chrono::NaiveDateTime> {
    match ttl {
        Some(ttl) => {
            let expires_at = chrono::Utc::now().naive_utc() + ttl;

            Some(expires_at)
        }
        None => None,
    }
}
