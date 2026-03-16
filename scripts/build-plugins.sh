#!/bin/bash
# ClawLegion Plugin Builder
# Builds all dynamic plugins independently and copies them to the output directory

set -e

# Get the script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PLUGINS_DIR="$PROJECT_ROOT/plugins"
OUTPUT_DIR="$PROJECT_ROOT/target/plugins"

# Platform-specific extension
case "$(uname -s)" in
    Darwin*)
        PLUGIN_EXT="dylib"
        ;;
    Linux*)
        PLUGIN_EXT="so"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLUGIN_EXT="dll"
        ;;
    *)
        PLUGIN_EXT="so"
        ;;
esac

echo "=== ClawLegion Plugin Builder ==="
echo "Project root: $PROJECT_ROOT"
echo "Plugins dir: $PLUGINS_DIR"
echo "Output dir: $OUTPUT_DIR"
echo "Platform: $(uname -s) (.$PLUGIN_EXT)"
echo ""

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Find and build all plugins
PLUGIN_COUNT=0
SUCCESS_COUNT=0

for plugin_dir in "$PLUGINS_DIR"/*/; do
    if [ -d "$plugin_dir" ]; then
        plugin_name=$(basename "$plugin_dir")
        plugin_cargo="$plugin_dir/Cargo.toml"

        if [ -f "$plugin_cargo" ]; then
            echo "Building plugin: $plugin_name"
            PLUGIN_COUNT=$((PLUGIN_COUNT + 1))

            # Build the plugin in release mode
            cd "$plugin_dir"
            if cargo build --release 2>&1; then
                # Find the built plugin file
                # The output name depends on the package name and platform
                target_dir="$plugin_dir/target/release"

                # Try different naming patterns
                # Pattern 1: lib{name}.dylib (macOS)
                # Pattern 2: lib{name}.so (Linux)
                # Pattern 3: {name}.dll (Windows)

                # Get the actual library name from Cargo.toml
                lib_name=$(grep "^name = " "$plugin_cargo" | head -1 | sed 's/name = "\(.*\)"/\1/' | tr '-' '_')

                if [ -z "$lib_name" ]; then
                    lib_name="$plugin_name"
                fi

                # Convert package name to library name (hyphens -> underscores)
                lib_name_underscore=$(echo "$lib_name" | tr '-' '_')

                # Try to find the built file
                plugin_file=""
                for pattern in "lib${lib_name_underscore}.$PLUGIN_EXT" "lib${plugin_name}.$PLUGIN_EXT"; do
                    if [ -f "$target_dir/$pattern" ]; then
                        plugin_file="$target_dir/$pattern"
                        break
                    fi
                done

                if [ -n "$plugin_file" ] && [ -f "$plugin_file" ]; then
                    # Copy to output directory
                    cp "$plugin_file" "$OUTPUT_DIR/"
                    echo "  -> Copied to $OUTPUT_DIR/$(basename "$plugin_file")"
                    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
                else
                    echo "  WARNING: Could not find built plugin file in $target_dir"
                    echo "  Looking for: lib${lib_name_underscore}.$PLUGIN_EXT"
                    ls -la "$target_dir" 2>/dev/null || true
                fi
            else
                echo "  ERROR: Failed to build $plugin_name"
            fi
            cd "$PROJECT_ROOT"
        fi
    fi
done

echo ""
echo "=== Build Summary ==="
echo "Total plugins found: $PLUGIN_COUNT"
echo "Successfully built: $SUCCESS_COUNT"

if [ $SUCCESS_COUNT -eq $PLUGIN_COUNT ] && [ $PLUGIN_COUNT -gt 0 ]; then
    echo ""
    echo "All plugins built successfully!"
    echo "Output directory: $OUTPUT_DIR"
    exit 0
elif [ $PLUGIN_COUNT -eq 0 ]; then
    echo ""
    echo "No plugins found in $PLUGINS_DIR"
    exit 1
else
    echo ""
    echo "Some plugins failed to build"
    exit 1
fi
