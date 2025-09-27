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

    # Create initial database if it doesn't exist
    if [ ! -f syspac.db.tar.gz ]; then
        echo "Creating new repository database..."
        sudo touch syspac.db.tar.gz syspac.files.tar.gz
        sudo ln -sf syspac.db.tar.gz syspac.db
        sudo ln -sf syspac.files.tar.gz syspac.files
        echo "Repository database initialized"
    else
        echo "Repository database already exists"
    fi

    cd - >/dev/null
}

# Try to download existing repository files
download_repo_files() {
    local dir="${REPO_ROOT}/x86_64"
    cd "${dir}"

    echo "Checking for existing repository files..."

    # Try to download database files, but don't fail if they don't exist
    for file in syspac.{db,files}{,.tar.gz}; do
        echo "Attempting to download ${file}..."
        curl -sSfL -o "${file}" \
             "https://github.com/${GITHUB_REPOSITORY}/releases/download/repository/${file}" || {
            echo "Note: ${file} not found in repository (this is normal for first run)"
        }
    done

    # Only try to download packages if we have a database
    if [ -s syspac.db.tar.gz ]; then
        echo "Found existing database, checking for packages..."
        # Extract package filenames from database
        tar -xf syspac.db.tar.gz -O | grep -A1 %FILENAME% | grep -v %FILENAME% | while read -r pkg; do
            if [ -n "$pkg" ]; then
                echo "Downloading ${pkg}..."
                curl -sSfL -o "${pkg}" \
                     "https://github.com/${GITHUB_REPOSITORY}/releases/download/repository/${pkg}" || {
                    echo "Warning: Failed to download ${pkg}"
                }
                if [ -n "${GPG_KEY_ID-}" ]; then
                    curl -sSfL -o "${pkg}.sig" \
                         "https://github.com/${GITHUB_REPOSITORY}/releases/download/repository/${pkg}.sig" || true
                fi
            fi
        done
    else
        echo "No existing database found, starting fresh repository"
    fi

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
update_repo_db() {
    local dir="${REPO_ROOT}/x86_64"
    cd "${dir}"

    echo "============================================"
    echo "Updating repository database"
    echo "============================================"

    # Check if we have any packages
    if ! compgen -G "*.pkg.tar.zst" >/dev/null; then
        echo "No packages found, skipping database update"
        return 0
    fi

    echo "Found packages:"
    ls -l *.pkg.tar.zst

    # Create temporary directory for database operations
    TEMP_DIR=$(mktemp -d)
    trap 'rm -rf "${TEMP_DIR}"' EXIT

    echo "Preparing packages for database update..."
    cp *.pkg.tar.zst "${TEMP_DIR}/"
    cd "${TEMP_DIR}"

    # Sign packages if GPG key is available
    if [ -n "${GPG_KEY_ID-}" ]; then
        echo "Signing packages..."
        for pkg in *.pkg.tar.zst; do
            if [ -f "${pkg}" ]; then
                echo "Signing ${pkg}..."
                gpg --detach-sign --use-agent -u "${GPG_KEY_ID}" "${pkg}"
            fi
        done
    fi

    # Create/update database
    echo "Updating package database..."
    if [ -n "${GPG_KEY_ID-}" ]; then
        repo-add -s -k "${GPG_KEY_ID}" -n -R syspac.db.tar.gz *.pkg.tar.zst
    else
        repo-add -n -R syspac.db.tar.gz *.pkg.tar.zst
    fi

    # Move everything back to the repository
    echo "Updating repository files..."
    sudo cp -fv *.pkg.tar.zst* syspac.{db,files}{,.tar.gz} "${dir}/"
    cd "${dir}"

    # Create symbolic links with proper permissions
    sudo ln -svf syspac.db.tar.gz syspac.db
    sudo ln -svf syspac.files.tar.gz syspac.files

    # Ensure correct permissions on all files
    sudo chown root:root *
    sudo chmod 644 *

    echo "Repository database update completed"
    cd - >/dev/null
}

# Build changed packages
if [ -n "${CHANGED_PACKAGES-}" ]; then
    echo "Processing packages: ${CHANGED_PACKAGES}"
    for pkg in ${CHANGED_PACKAGES}; do
        if [ -d "/build/${pkg}" ]; then
            if ! build_package "/build/${pkg}"; then
                echo "ERROR: Failed to build package ${pkg}"
                exit 1
            fi
        else
            echo "WARNING: Package directory ${pkg} not found"
        fi
    done
else
    echo "No packages to build"
fi

# Update repository database
update_repo_db

echo "============================================"
echo "Build process completed successfully at $(date)"
echo "============================================"
