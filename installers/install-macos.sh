#!/usr/bin/env bash
# KERNEXT macOS installer
# Installs KERNEXT to /usr/local/bin or ~/.local/bin.

set -euo pipefail

VERSION="${KERNEXT_VERSION:-0.1.0}"
REPO_URL="https://github.com/Trishit6/terminal"
INSTALL_DIR="/usr/local/bin"
BINARY_NAME="kernext"
ARCHIVE_NAME="kernext-x86_64-apple-darwin.tar.gz"
ARCHIVE_URL="${REPO_URL}/releases/download/v${VERSION}/${ARCHIVE_NAME}"

# Animation is presentation only. It is disabled automatically for pipes,
# redirected output, NO_COLOR, and dumb terminals.
if [[ -t 1 && -z "${NO_COLOR:-}" && "${TERM:-}" != "dumb" ]]; then
    UI=1
else
    UI=0
fi

paint() {
    local code=$1
    shift
    if [[ $UI -eq 1 ]]; then
        printf '\033[%sm%s\033[0m' "$code" "$*"
    else
        printf '%s' "$*"
    fi
}

print_info() { printf '%s %s\n' "$(paint '38;5;39' '[INFO]')" "$*"; }
print_success() { printf '%s %s\n' "$(paint '38;5;42' '[ OK ]')" "$*"; }
print_warning() { printf '%s %s\n' "$(paint '38;5;214' '[WARN]')" "$*"; }
print_error() { printf '%s %s\n' "$(paint '38;5;203' '[FAIL]')" "$*" >&2; }

LOGO_LINES=(
    '╦╔═╔═╗╦═╗╔╗╔╔═╗═╗╔╦'
    '╠╩╗║╣ ╠╦╝║║║║╣  ╠╩╗║'
    '╩ ╩╚═╝╩╚═╝╚╝╚═╝═╝ ╩'
)
PALETTE=(39 45 51 99 63 69 75 81)

paint_logo_frame() {
    local offset=$1
    local row color
    for row in 0 1 2; do
        color=${PALETTE[$(((row + offset) % ${#PALETTE[@]}))]}
        printf '\033[38;5;%sm%s\033[0m\n' "$color" "${LOGO_LINES[$row]}"
    done
}

print_banner() {
    printf '\n'
    if [[ $UI -eq 1 ]]; then
        local frame
        for ((frame = 0; frame < 20; frame++)); do
            paint_logo_frame "$frame"
            if ((frame + 1 < 20)); then
                printf '\033[3A'
            fi
            sleep 0.03
        done
    else
        printf '%s\n' "${LOGO_LINES[@]}"
    fi

    printf '\n%s\n' "$(paint '1;38;5;51' 'KERNEXT') $(paint '38;5;245' "v${VERSION}")"
    printf '%s\n\n' "$(paint '38;5;111' 'The Next Core of Computing.')"
}

SPINNER=(⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏)

run_with_spinner() {
    local label=$1
    shift

    if [[ $UI -eq 0 ]]; then
        "$@"
        return $?
    fi

    local index=0
    "$@" &
    local pid=$!

    while kill -0 "$pid" 2>/dev/null; do
        local color=${PALETTE[$((index % ${#PALETTE[@]}))]}
        printf '\r\033[K\033[38;5;%sm%s\033[0m %s' \
            "$color" "${SPINNER[$((index % ${#SPINNER[@]}))]}" "$label" >&2
        index=$((index + 1))
        sleep 0.08
    done

    wait "$pid"
    local status=$?
    printf '\r\033[K' >&2
    return "$status"
}

print_banner

if [[ "$(uname)" != "Darwin" ]]; then
    print_error "This installer is for macOS only."
    exit 1
fi

if [[ $EUID -eq 0 ]]; then
    print_warning "Running as root. Will install to ${INSTALL_DIR}."
else
    if [[ ! -w "${INSTALL_DIR}" ]]; then
        print_info "Installing to ~/.local/bin instead (no root required)."
        INSTALL_DIR="${HOME}/.local/bin"
        mkdir -p "${INSTALL_DIR}"
        if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
            print_warning "Adding ${INSTALL_DIR} to PATH in ~/.zprofile."
            echo 'export PATH="${HOME}/.local/bin:${PATH}"' >> "${HOME}/.zprofile"
        fi
    fi
fi

print_info "Install directory: ${INSTALL_DIR}"

TMPDIR=$(mktemp -d)
trap 'rm -rf "${TMPDIR}"' EXIT
cd "${TMPDIR}"

if ! run_with_spinner "Downloading ${ARCHIVE_NAME}" \
    curl -fsSL "${ARCHIVE_URL}" -o "${ARCHIVE_NAME}"; then
    print_error "Failed to download ${ARCHIVE_URL}"
    print_info "You can manually download from: ${REPO_URL}/releases"
    exit 1
fi
print_success "Downloaded KERNEXT v${VERSION}."

if curl -fsSL "${ARCHIVE_URL}.sha256" -o "${ARCHIVE_NAME}.sha256" 2>/dev/null; then
    if run_with_spinner "Verifying checksum" \
        shasum -a 256 -c "${ARCHIVE_NAME}.sha256"; then
        print_success "Checksum verified."
    else
        print_error "Checksum verification failed."
        exit 1
    fi
else
    print_warning "No published checksum found; skipping verification."
fi

if ! run_with_spinner "Extracting archive" tar -xzf "${ARCHIVE_NAME}"; then
    print_error "Could not extract ${ARCHIVE_NAME}."
    exit 1
fi

if [[ ! -f "${BINARY_NAME}" ]]; then
    print_error "Binary not found in archive."
    exit 1
fi

if ! run_with_spinner "Installing to ${INSTALL_DIR}/${BINARY_NAME}" \
    install -m 755 "${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"; then
    print_error "Could not install to ${INSTALL_DIR}. Try running with sudo."
    exit 1
fi

print_success "KERNEXT installed successfully."
printf '%s\n\n' "$(paint '38;5;111' 'Build. Execute. Innovate.')"
print_info "Run 'kernext' to start the shell."
print_info "Run 'kernext --help' for usage information."

if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
    print_warning "${INSTALL_DIR} is not in your PATH."
    print_info "Add it with: export PATH=\"${INSTALL_DIR}:\${PATH}\""
fi
