#!/bin/bash

# Default configuration
GITHUB_REPO=${GITHUB_REPO:-"cestef/braise"}
SKIP_VERSION_CHECK=${SKIP_VERSION_CHECK:-false}
SPECIFIC_VERSION=${SPECIFIC_VERSION:-""}
NO_COLORS=${NO_COLORS:-false}
INCLUDE_DEV=${INCLUDE_DEV:-false}
VERBOSE=${VERBOSE:-false}

PROJECT_NAME=$(basename "$GITHUB_REPO")
INSTALLER_NAME="${PROJECT_NAME}-installer.sh"

# Color setup
if [ "$NO_COLORS" = true ]; then
    C_BOLD="" C_DIM="" C_GREEN="" C_RED="" C_YELLOW="" C_BLUE="" C_CYAN="" C_RESET=""
else
    C_BOLD="\033[1m"
    C_DIM="\033[2m"
    C_GREEN="\033[0;32m"
    C_RED="\033[0;31m"
    C_YELLOW="\033[0;33m"
    C_BLUE="\033[0;34m"
    C_CYAN="\033[0;36m"
    C_RESET="\033[0m"
fi

# Logging functions
msg() {
    local color="$1"
    local message="$2"
    local level="${3:-}"
    
    if [ -n "$level" ]; then
        echo -e "${color}[${level}] ${message}${C_RESET}"
    else
        echo -e "${color}${message}${C_RESET}"
    fi
}

verbose() {
    if [ "$VERBOSE" = true ]; then
        msg "$C_DIM" "$1" "VERBOSE"
    fi
}

show_help() {
    
echo -e "${C_BOLD}GitHub Release Installer${C_RESET}

${C_BOLD}USAGE:${C_RESET}
    $0 [OPTIONS]

${C_BOLD}OPTIONS:${C_RESET}
    -r, --repo REPO         GitHub repository (format: owner/repo)
                           Default: $GITHUB_REPO
    -v, --version VERSION   Install specific version (e.g., v1.2.3)
    -d, --dev              Include development/pre-release versions
    -s, --skip-version     Skip version check and use latest available
    -n, --no-colors        Disable colored output
    --verbose              Enable verbose logging
    -h, --help             Show this help message

${C_BOLD}ENVIRONMENT VARIABLES:${C_RESET}
    GITHUB_REPO            Same as --repo
    SPECIFIC_VERSION       Same as --version
    INCLUDE_DEV            Same as --dev (true/false)
    SKIP_VERSION_CHECK     Same as --skip-version (true/false)
    NO_COLORS              Same as --no-colors (true/false)
    VERBOSE                Same as --verbose (true/false)

${C_BOLD}EXAMPLES:${C_RESET}
    # Install latest stable release
    $0

    # Install latest including dev releases
    $0 --dev

    # Install specific version
    $0 --version v1.2.3

    # Install from different repository
    $0 --repo user/project --dev

    # Verbose installation with no colors
    $0 --verbose --no-colors"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--repo)
            GITHUB_REPO="$2"
            PROJECT_NAME=$(basename "$GITHUB_REPO")
            INSTALLER_NAME="${PROJECT_NAME}-installer.sh"
            shift 2
            ;;
        -v|--version)
            SPECIFIC_VERSION="$2"
            shift 2
            ;;
        -d|--dev)
            INCLUDE_DEV=true
            shift
            ;;
        -s|--skip-version)
            SKIP_VERSION_CHECK=true
            shift
            ;;
        -n|--no-colors)
            NO_COLORS=true
            # Redefine colors as empty
            C_BOLD="" C_DIM="" C_GREEN="" C_RED="" C_YELLOW="" C_BLUE="" C_CYAN="" C_RESET=""
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        -h|--help)
            show_help
            exit 0
            ;;
        *)
            msg "$C_RED" "Unknown option: $1" "ERROR"
            echo "Use --help for usage information."
            exit 1
            ;;
    esac
done

verbose "Configuration:"
verbose "  Repository: $GITHUB_REPO"
verbose "  Project: $PROJECT_NAME"
verbose "  Include dev: $INCLUDE_DEV"
verbose "  Skip version check: $SKIP_VERSION_CHECK"
verbose "  Specific version: ${SPECIFIC_VERSION:-"(none)"}"

echo -e "${C_BOLD}Installing $PROJECT_NAME...${C_RESET}"

get_latest_version() {
    local api_url="https://api.github.com/repos/$GITHUB_REPO/releases"
    
    if [ "$INCLUDE_DEV" = true ]; then
        verbose "Fetching all releases (including pre-releases)..."
        # Get the first release (latest including pre-releases)
        curl -s "$api_url" | grep '"tag_name":' | head -1 | sed -E 's/.*"([^"]+)".*/\1/'
    else
        verbose "Fetching latest stable release..."
        # Get latest non-prerelease
        curl -s "${api_url}/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    fi
}

if [ -z "$SPECIFIC_VERSION" ] && [ "$SKIP_VERSION_CHECK" != "true" ]; then
    echo -e "${C_DIM}Fetching version information...${C_RESET}"
    
    LATEST_VERSION=$(get_latest_version)
    
    if [ -z "$LATEST_VERSION" ]; then
        msg "$C_RED" "Failed to fetch version from GitHub API" "ERROR"
        msg "$C_YELLOW" "Repository: $GITHUB_REPO" "INFO"
        exit 1
    fi
    
    # Check if it's a dev release
    if echo "$LATEST_VERSION" | grep -qE "(dev|alpha|beta|rc|pre)" || [ "$INCLUDE_DEV" = true ]; then
        if [ "$INCLUDE_DEV" = true ]; then
            msg "$C_CYAN" "Latest version (including dev): ${C_BOLD}$LATEST_VERSION${C_RESET}" "SUCCESS"
        else
            msg "$C_YELLOW" "Latest version contains dev/pre-release tag: ${C_BOLD}$LATEST_VERSION${C_RESET}" "WARNING"
            msg "$C_DIM" "Use --dev flag to explicitly include development releases" "INFO"
        fi
    else
        msg "$C_GREEN" "Latest stable version: ${C_BOLD}$LATEST_VERSION${C_RESET}" "SUCCESS"
    fi
    
    VERSION_TO_INSTALL=$LATEST_VERSION
else
    VERSION_TO_INSTALL=$SPECIFIC_VERSION
    if [ -n "$SPECIFIC_VERSION" ]; then
        msg "$C_YELLOW" "Using specified version: ${C_BOLD}$VERSION_TO_INSTALL${C_RESET}" "VERSION"
    else
        echo -e "${C_DIM}Skipping version check, using latest available${C_RESET}"
    fi
fi

# Validate version format (should start with 'v' typically)
if [ -n "$VERSION_TO_INSTALL" ] && ! echo "$VERSION_TO_INSTALL" | grep -qE "^v?[0-9]"; then
    msg "$C_YELLOW" "Warning: Version '$VERSION_TO_INSTALL' doesn't follow typical semver format" "WARNING"
fi

echo -e "${C_BLUE}Downloading installer...${C_RESET}"
INSTALLER_URL="https://github.com/$GITHUB_REPO/releases/download/${VERSION_TO_INSTALL}/$INSTALLER_NAME"
verbose "Installer URL: $INSTALLER_URL"

TEMP_INSTALLER=$(mktemp)
verbose "Temporary file: $TEMP_INSTALLER"

echo -e "${C_DIM}From: $INSTALLER_URL${C_RESET}"

# Download with progress if verbose, silent otherwise
if [ "$VERBOSE" = true ]; then
    curl --proto '=https' --tlsv1.2 -L --progress-bar "$INSTALLER_URL" -o "$TEMP_INSTALLER"
else
    curl --proto '=https' --tlsv1.2 -LsSf "$INSTALLER_URL" -o "$TEMP_INSTALLER"
fi

DOWNLOAD_STATUS=$?

if [ $DOWNLOAD_STATUS -eq 0 ]; then
    verbose "Download successful, making installer executable"
    chmod +x "$TEMP_INSTALLER"
    
    msg "$C_BLUE" "Running installer..."
    
    # Run installer with appropriate shell
    if command -v bash >/dev/null 2>&1; then
        verbose "Using bash to run installer"
        bash "$TEMP_INSTALLER"
    else
        verbose "Using sh to run installer"
        sh "$TEMP_INSTALLER"
    fi
    
    INSTALL_STATUS=$?
    
    verbose "Cleaning up temporary file"
    rm "$TEMP_INSTALLER"
    
    if [ $INSTALL_STATUS -eq 0 ]; then
        msg "$C_GREEN" "Installation completed successfully!" "SUCCESS"
    else
        msg "$C_RED" "Installation failed with exit code $INSTALL_STATUS" "ERROR"
    fi
    
    exit $INSTALL_STATUS
else
    case $DOWNLOAD_STATUS in
        22)
            msg "$C_RED" "Installer not found (HTTP 404)" "ERROR"
            msg "$C_YELLOW" "The release might not have an installer file named '$INSTALLER_NAME'" "INFO"
            ;;
        6)
            msg "$C_RED" "Could not resolve host" "ERROR"
            ;;
        7)
            msg "$C_RED" "Failed to connect to host" "ERROR"
            ;;
        *)
            msg "$C_RED" "Failed to download installer (curl exit code: $DOWNLOAD_STATUS)" "ERROR"
            ;;
    esac
    
    verbose "Cleaning up temporary file"
    rm "$TEMP_INSTALLER"
    exit 1
fi