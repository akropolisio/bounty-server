#!/usr/bin/env bash

rm ./test.sqlite

diesel database reset --migration-dir ./migrations-sqlite
# diesel setup --migration-dir ./migrations-sqlite
diesel migration run --migration-dir ./migrations-sqlite
diesel migration redo --migration-dir ./migrations-sqlite
diesel migration redo --migration-dir ./migrations-sqlite


echo "Maybe you wanna git checkout -- src/db/schema.rs"
