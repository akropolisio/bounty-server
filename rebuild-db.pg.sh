#!/usr/bin/env bash

diesel database reset
# diesel setup
diesel migration run
diesel migration redo
diesel migration redo


echo "Maybe you wanna git checkout -- src/db/schema.rs"
