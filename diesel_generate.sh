
# cargo binstall diesel_cli

# add new migration = database version = empty set of up.sql + down.sql
diesel migration generate add_peer_nicknames

# generates schema.rs:
diesel migration run

# verify up.sql + down.sql symmetry
#diesel migration redo

# feature sets of LukasLohmar & abbychau merged:
#cargo install --git "https://github.com/thomas725/diesel_cli_ext" --branch master
# todo: switch to upstream once it supports "struct-attribute" parameters or an equivalent.
# see also: https://github.com/abbychau/diesel_cli_ext/issues/67
#cargo install --git "https://github.com/LukasLohmar/diesel_cli_ext" --branch dev
# diesel_ext --import-types "crate::schema::*" --add-table-name --struct-attribute "#[diesel(check_for_backend(diesel::sqlite::Sqlite))]" --derive "Queryable, Selectable, Debug" > src/models_queryable.rs

#cargo install diesel_cli_ext
# diesel_ext --import-types "crate::schema::*" --import-types "diesel::Insertable" --insertable > src/models_insertable.rs
# WARNING: needs manual modification: delete <'a> from structs without borrowed data.
