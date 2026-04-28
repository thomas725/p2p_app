#!/usr/bin/env bash
set -e

# Diesel Model Generator with Automatic Type Conversion
#
# This script regenerates Diesel models from database schema with automatic
# post-processing to convert borrowed reference types (&'a str) to owned String
# types in insertable models. This simplifies lifetime management and makes
# the generated models immediately usable without further modifications.
#
# Usage: ./diesel_generate.sh
#
# After modifying migrations:
#   1. Update migrations/<timestamp>_description/up.sql and down.sql
#   2. Run: ./diesel_generate.sh
#   3. Commit changes - no manual editing needed!

echo "=== Diesel Model Generation with Type Conversion ==="

# Step 1: Run migrations to update the database schema
echo ""
echo "Step 1: Running migrations..."
diesel migration run

# Step 2: Generate schema.rs from database
echo ""
echo "Step 2: Generating schema.rs..."
diesel print-schema > src/generated/schema.rs

# Step 3: Generate queryable models
echo ""
echo "Step 3: Generating queryable models (models_queryable.rs)..."
diesel_ext \
  --import-types "crate::generated::schema::*" \
  --add-table-name \
  --derive "Queryable, Selectable, Debug, Clone" \
  > src/generated/models_queryable.rs

# Step 4: Generate insertable models (raw output)
echo ""
echo "Step 4: Generating insertable models (raw from diesel_ext)..."
diesel_ext \
  --import-types "crate::generated::schema::*" \
  --insertable \
  --derive "Insertable, Debug" \
  > /tmp/models_insertable_raw.rs

# Step 5: Post-process to convert borrowed types to owned types
echo ""
echo "Step 5: Post-processing (converting &'a str to String for ergonomics)..."

# Transform diesel_ext output:
#   pub struct NewFoo<'a> { pub field: &'a str, ... }
# Into:
#   pub struct NewFoo { pub field: String, ... }
#
# This eliminates lifetime complexity in calling code while maintaining type safety.

sed \
  -e "s/#\[derive(Insertable)\]/#[derive(Insertable, Debug)]/g" \
  -e "s/pub struct New\([A-Za-z]*\)<'a>/pub struct New\1/g" \
  -e "s/Option<&'a str>/Option<String>/g" \
  -e "s/&'a str/String/g" \
  -e "s/<'a>//g" \
  -e "/^use crate::generated::schema::\*;$/a use diesel::Insertable;" \
  /tmp/models_insertable_raw.rs > src/generated/models_insertable.rs

# Cleanup temporary file
rm /tmp/models_insertable_raw.rs

# Step 6: Verify the build works
echo ""
echo "Step 6: Verifying build..."
if ! cargo build 2>&1 | tail -5; then
    echo ""
    echo "ERROR: Build failed after model generation!"
    echo "Please check the errors above."
    exit 1
fi

# Step 7: Run tests to ensure models work correctly
echo ""
echo "Step 7: Running tests..."
if ! cargo test --lib --test tui_chat --test p2p_integration 2>&1 | tail -10; then
    echo ""
    echo "WARNING: Some tests failed after model generation."
    echo "Check the output above for details."
    exit 1
fi

echo ""
echo "SUCCESS: Models regenerated and validated!"
echo ""
echo "Generated files:"
echo "  - src/generated/schema.rs"
echo "  - src/generated/models_queryable.rs"
echo "  - src/generated/models_insertable.rs"
echo ""
echo "Insertable models use owned String types for simplicity and ergonomics."
echo "No manual editing required - commit these changes directly!"
