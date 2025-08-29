
# cargo binstall diesel_cli
# generates schema.rs:
diesel migration run

# verify up.sql + down.sql symmetry
#diesel migration redo

# cargo install --git "https://github.com/LukasLohmar/diesel_cli_ext" --branch dev
diesel_ext --import-types "crate::schema::*" --add-table-name --struct-attribute "#[diesel(check_for_backend(diesel::sqlite::Sqlite))]" --derive "Queryable, Selectable, Debug" > src/models_generated.rs