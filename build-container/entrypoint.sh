#!/bin/bash
set -euo pipefail

# Enable logging and error tracing
exec 1> >(tee "/repo/build-output.txt") 2>&1
set -x
echo "Build process started at $(date)"

# Import GPG key if provided
if [ -n "${GPG_KEY_DATA-}" ]; then
    if [ -z "${GPG_KEY_ID-}" ]; then
        echo 'ERROR: GPG_KEY_ID is required when GPG_KEY_DATA is provided'
        exit 1
    fi
    echo "Importing GPG key..."
    gpg --import --no-tty <<<"${GPG_KEY_DATA}"
    echo "Trusting GPG key..."
    gpg --import-ownertrust --no-tty <<<"${GPG_KEY_ID}:5:"
fi

# Set up repository directory
REPO_ROOT="/repo"
echo "Creating repository directory structure..."
sudo mkdir -p "${REPO_ROOT}/x86_64"

# Initialize repository if needed
initialize_repo() {
    local dir="${REPO_ROOT}/x86_64"
    echo "Initializing repository at ${dir}..."
    cd "${dir}"

    # Ensure directory exists, database is now managed by the GitHub Actions workflow
    if [ ! -d "${dir}" ]; then
        echo "Creating repository directory..."
        sudo mkdir -p "${dir}"
    fi

    cd - >/dev/null
}

# Try to download existing repository files (packages + DB)
# Note: DB and pruning are now managed by the GitHub Actions workflow;
# this function only ensures that existing packages/DB are available inside the container.
download_repo_files() {
    local dir="${REPO_ROOT}/x86_64"
    cd "${dir}"

    echo "Checking for existing repository files..."

    # Determine release tag (default to 'repo' if not provided)
    local release_tag="${RELEASE_TAG:-repo}"

    # Download database files if they exist, but don't fail if they don't
    for file in syspac.{db,files}{,.tar.gz}; do
        echo "Attempting to download ${file} from tag ${release_tag}..."
        curl -sSfL -o "${file}" \
             "https://github.com/${GITHUB_REPOSITORY}/releases/download/${release_tag}/${file}" || {
            echo "Note: ${file} not found in repository (this is normal for first run)"
        }
    done

    # Download existing package files directly from the release
    echo "Attempting to download existing packages from tag ${release_tag}..."
    curl -sSfL \
         "https://github.com/${GITHUB_REPOSITORY}/releases/download/${release_tag}/" \
         || echo "Note: unable to list release contents directly (this is expected without index)"

    # Best-effort download of common package patterns; pruning is handled by the workflow
    for pattern in "*.pkg.tar.zst" "*.pkg.tar.xz" "*.pkg.tar.gz"; do
        echo "Attempting to download packages matching ${pattern}..."
        # gh CLI is not available inside the container; rely on workflow-side downloads instead.
        # This loop remains for compatibility but will typically be a no-op.
        :
    done

    cd - >/dev/null
}

# Initialize repository structure
initialize_repo
download_repo_files

# Function to build a package
build_package() {
    local pkg_dir=$1
    local pkg_name=$(basename "${pkg_dir}")
    local build_log="${REPO_ROOT}/${pkg_name}-build.log"

    echo "============================================"
    echo "Starting build for ${pkg_name}"
    echo "Build log: ${build_log}"
    echo "============================================"

    # Create build directory
    echo "Preparing build environment..."
    sudo mkdir -p "/build/build/${pkg_name}"
    sudo cp -r "${pkg_dir}"/* "/build/build/${pkg_name}/"
    sudo chown -R builder:builder "/build/build/${pkg_name}"
    cd "/build/build/${pkg_name}"

    # Build package with detailed logging
    {
        echo "Build initiated at $(date)"
        echo "Package: ${pkg_name}"
        echo "Working directory: $(pwd)"
        echo "Content of PKGBUILD:"
        echo "----------------------------------------"
        cat PKGBUILD
        echo "----------------------------------------"
        echo "Starting makepkg..."

        # Actually build the package
        if ! makepkg -s --noconfirm; then
            echo "ERROR: Package build failed!"
            return 1
        fi

        echo "Build completed successfully"
        echo "Built files:"
        ls -l *.pkg.tar.zst || echo "No package files found!"
        echo "----------------------------------------"
        echo "Build completed at $(date)"
    } 2>&1 | tee "${build_log}"

    # Move built packages to repository
    echo "Moving built packages to repository..."
    sudo mkdir -p "${REPO_ROOT}/x86_64"
    if ! sudo find . -name "*.pkg.tar.zst" -exec sudo mv -v {} "${REPO_ROOT}/x86_64/" \;; then
        echo "ERROR: Failed to move built packages!"
        return 1
    fi

    echo "Package build process completed"
    cd - >/dev/null
}

# Function to update repository database
# NOTE: In CI, the GitHub Actions workflow performs pruning of obsolete packages
# and a full DB rebuild from the current set of *.pkg.tar.* files. This function
# is retained only for potential non-CI/manual use.
update_repo_db() {
    local dir="${REPO_ROOT}/x86_64"
    cd "${dir}"

    echo "============================================"
    echo "Updating repository database (simple mode)"
    echo "============================================"

    # Check if we have any packages
    if ! compgen -G "*.pkg.tar.*" >/dev/null; then
        echo "No packages found, skipping database update"
        return 0
    fi

    echo "Found packages:"
    ls -l *.pkg.tar.*

    # Sign packages if GPG key is available
    if [ -n "${GPG_KEY_ID-}" ]; then
        echo "Signing packages..."
        for pkg in *.pkg.tar.*; do
            if [ -f "${pkg}" ]; then
                echo "Signing ${pkg}..."
                gpg --detach-sign --use-agent -u "${GPG_KEY_ID}" "${pkg}" || true
            fi
        done
    fi

    # Create/update database based on current files
    echo "Rebuilding package database from current package files..."
    rm -f syspac.db* syspac.files* syspac.db.tar.gz.lck || true
    if [ -n "${GPG_KEY_ID-}" ]; then
        repo-add -s -k "${GPG_KEY_ID}" -n -R syspac.db.tar.gz ./*.pkg.tar.*
    else
        repo-add -n -R syspac.db.tar.gz ./*.pkg.tar.*
    fi

    # Create symbolic links with proper permissions
    sudo ln -svf syspac.db.tar.gz syspac.db
    sudo ln -svf syspac.files.tar.gz syspac.files

    echo "Repository database update completed (simple mode)"
    cd - >/dev/null
}

# Build changed packages
# CHANGED_PACKAGES is expected to contain paths relative to the repo root
# (e.g. "packages/foo"), as provided by `syspac detect-changes --paths`.
if [ -n "${CHANGED_PACKAGES-}" ]; then
    echo "Processing packages (paths): ${CHANGED_PACKAGES}"
    for pkg in ${CHANGED_PACKAGES}; do
        if [ -d "/build/${pkg}" ]; then
            echo "Building package from /build/${pkg}"
            if ! build_package "/build/${pkg}"; then
                echo "ERROR: Failed to build package ${pkg}"
                exit 1
            fi
        else
            echo "WARNING: Package directory /build/${pkg} not found; skipping"
        fi
    done
else
    echo "No packages to build"
fi

# Update repository database
# In CI, the GitHub Actions workflow performs pruning and a full DB rebuild.
# Skip the container-side DB update in CI to avoid lock/permission issues and
# rely solely on the outer workflow for the production repository state.
if [ "${GITHUB_ACTIONS-}" != "true" ]; then
    update_repo_db
else
    echo "Skipping container-side update_repo_db in CI; outer workflow will rebuild DB."
fi

echo "============================================"
echo "Build process completed successfully at $(date)"
echo "============================================"
