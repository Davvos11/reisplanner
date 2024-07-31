use rbatis::RBatis;

/// Get database connection.
pub fn new_db_connection() -> anyhow::Result<RBatis> {
    let rb = RBatis::new();
    let project_dir = env!("CARGO_MANIFEST_DIR");
    if project_dir.is_empty() {
        panic!("CARGO_MANIFEST_DIR env variable is not set.\
        It should be set to the `reisplanner-algorithm` folder")
    }
    rb.init(
        rbdc_sqlite::driver::SqliteDriver {},
        format!("sqlite://{project_dir}/../sqlite.db").as_str(),
    )?;
    Ok(rb)
}

