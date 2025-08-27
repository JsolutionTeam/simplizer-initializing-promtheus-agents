#!/bin/bash

# Release helper script
# Usage: ./release.sh [version]
# Example: ./release.sh 1.0.0

set -e

VERSION=${1}

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 1.0.0"
    exit 1
fi

# Check if version starts with 'v'
if [[ ! "$VERSION" =~ ^v ]]; then
    VERSION="v${VERSION}"
fi

echo "Creating release for version: ${VERSION}"

# Update Cargo.toml version (remove 'v' prefix for cargo)
CARGO_VERSION=${VERSION#v}
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/^version = .*/version = \"${CARGO_VERSION}\"/" Cargo.toml
else
    # Linux
    sed -i "s/^version = .*/version = \"${CARGO_VERSION}\"/" Cargo.toml
fi

# Commit version change
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to ${VERSION}" || true

# Create and push tag
git tag -a "${VERSION}" -m "Release ${VERSION}"

echo "Tag ${VERSION} created successfully!"
echo ""
echo "To trigger the release, push the tag to GitHub:"
echo "  git push origin ${VERSION}"
echo ""
echo "Or push with commits:"
echo "  git push origin main && git push origin ${VERSION}"