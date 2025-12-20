#!/bin/bash
#
# ntrip-core Comprehensive Test Suite
#
# Exercises the NTRIP client against various public casters
# with different protocols, ports, and authentication methods.
#
# CREDENTIAL HANDLING:
# 1. Create a file: scripts/credentials.env (gitignored)
# 2. Or export environment variables before running
#
# Example credentials.env:
#   export NTRIP_RTK2GO_USER="your.email@example.com"
#   export NTRIP_RTK2GO_PASS="none"
#   export NTRIP_AUSCORS_USER="your_username"
#   export NTRIP_AUSCORS_PASS="your_password"
#
# Usage:
#   ./scripts/test_suite.sh [--quick|--full|--connect|--all]
#
#   --quick    Sourcetable tests only (no auth required)
#   --full     Sourcetable + nearest tests
#   --connect  Stream connection tests (requires credentials)
#   --all      All tests including stress tests

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counters
PASS=0
FAIL=0
SKIP=0

# Load credentials from file if it exists
CREDS_FILE="$SCRIPT_DIR/credentials.env"
if [[ -f "$CREDS_FILE" ]]; then
    echo -e "${BLUE}Loading credentials from $CREDS_FILE${NC}"
    source "$CREDS_FILE"
else
    echo -e "${YELLOW}No credentials file found at $CREDS_FILE${NC}"
    echo -e "${YELLOW}Stream connection tests will be skipped unless env vars are set${NC}"
fi

# Test locations - override in credentials file or env
TEST_LAT_AU="${TEST_LAT_AU:--27.47}"      # Brisbane, Australia
TEST_LON_AU="${TEST_LON_AU:-153.02}"
TEST_LAT_EU="${TEST_LAT_EU:-52.52}"       # Berlin, Germany  
TEST_LON_EU="${TEST_LON_EU:-13.41}"
TEST_LAT_FR="${TEST_LAT_FR:-48.86}"       # Paris, France
TEST_LON_FR="${TEST_LON_FR:-2.35}"
TEST_LAT_NA="${TEST_LAT_NA:-40.71}"       # New York, USA
TEST_LON_NA="${TEST_LON_NA:--74.00}"

# Build examples
echo -e "\n${BLUE}=== Building examples ===${NC}\n"
cd "$PROJECT_DIR"
cargo build --examples --release 2>/dev/null || cargo build --examples

# Determine binary paths
if [[ -f "$PROJECT_DIR/target/release/examples/sourcetable" ]]; then
    BIN_DIR="$PROJECT_DIR/target/release/examples"
else
    BIN_DIR="$PROJECT_DIR/target/debug/examples"
fi

SOURCETABLE="$BIN_DIR/sourcetable"
NEAREST="$BIN_DIR/nearest"
CONNECT="$BIN_DIR/connect"

# Helper functions
run_test() {
    local name="$1"
    shift
    local cmd="$@"
    
    echo -e "\n${BLUE}[$name]${NC}"
    echo -e "  Command: $cmd"
    
    if timeout 30 $cmd > /tmp/ntrip_test_output.txt 2>&1; then
        echo -e "  ${GREEN}PASS${NC}"
        ((PASS++))
        # Show brief output
        head -5 /tmp/ntrip_test_output.txt | sed 's/^/    /'
        return 0
    else
        local exit_code=$?
        echo -e "  ${RED}FAIL${NC} (exit code: $exit_code)"
        ((FAIL++))
        # Show error output
        tail -10 /tmp/ntrip_test_output.txt | sed 's/^/    /'
        return 1
    fi
}

skip_test() {
    local name="$1"
    local reason="$2"
    echo -e "\n${YELLOW}[$name] SKIP${NC}: $reason"
    ((SKIP++))
}

print_summary() {
    echo -e "\n${BLUE}========================================${NC}"
    echo -e "${BLUE}           TEST SUMMARY${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo -e "  ${GREEN}PASS${NC}: $PASS"
    echo -e "  ${RED}FAIL${NC}: $FAIL"
    echo -e "  ${YELLOW}SKIP${NC}: $SKIP"
    echo -e "  Total: $((PASS + FAIL + SKIP))"
    echo -e "${BLUE}========================================${NC}\n"
    
    if [[ $FAIL -gt 0 ]]; then
        return 1
    fi
    return 0
}

# =============================================================================
# SOURCETABLE TESTS (No auth required)
# =============================================================================
run_sourcetable_tests() {
    echo -e "\n${BLUE}=== SOURCETABLE TESTS ===${NC}"
    
    # RTK2go - Large public caster
    run_test "RTK2go sourcetable" "$SOURCETABLE" rtk2go.com 2101
    
    # EUREF - European reference stations
    run_test "EUREF sourcetable" "$SOURCETABLE" euref-ip.net 2101
    
    # Centipede - French open community
    run_test "Centipede sourcetable" "$SOURCETABLE" caster.centipede.fr 2101
    
    # IGS - International GNSS Service
    run_test "IGS sourcetable" "$SOURCETABLE" igs-ip.net 2101
}

# =============================================================================
# NEAREST MOUNTPOINT TESTS
# =============================================================================
run_nearest_tests() {
    echo -e "\n${BLUE}=== NEAREST MOUNTPOINT TESTS ===${NC}"
    
    # Test from different global locations
    run_test "Nearest (Brisbane AU)" "$NEAREST" rtk2go.com "$TEST_LAT_AU" "$TEST_LON_AU"
    run_test "Nearest (Berlin EU)" "$NEAREST" rtk2go.com "$TEST_LAT_EU" "$TEST_LON_EU"
    run_test "Nearest (New York NA)" "$NEAREST" rtk2go.com "$TEST_LAT_NA" "$TEST_LON_NA"
    
    # EUREF from Europe
    run_test "Nearest EUREF (Berlin)" "$NEAREST" euref-ip.net "$TEST_LAT_EU" "$TEST_LON_EU"
    
    # Centipede from France
    run_test "Nearest Centipede (Paris)" "$NEAREST" caster.centipede.fr "$TEST_LAT_FR" "$TEST_LON_FR"
}

# =============================================================================
# CONNECTION TESTS (Require credentials)
# =============================================================================
run_connection_tests() {
    echo -e "\n${BLUE}=== CONNECTION TESTS ===${NC}"
    
    # RTK2go (public, requires email as username, "none" as password)
    if [[ -n "$NTRIP_RTK2GO_USER" ]]; then
        # Find a mountpoint near the test location
        echo -e "${BLUE}Finding nearest RTK2go mountpoint...${NC}"
        local mountpoint=$("$NEAREST" rtk2go.com "$TEST_LAT_NA" "$TEST_LON_NA" 2>/dev/null | grep "Recommended:" | awk '{print $2}')
        
        if [[ -n "$mountpoint" ]]; then
            run_test "RTK2go connect ($mountpoint)" \
                "$CONNECT" rtk2go.com "$mountpoint" 2101 \
                --user="$NTRIP_RTK2GO_USER" --pass="${NTRIP_RTK2GO_PASS:-none}"
        else
            skip_test "RTK2go connect" "Could not find nearby mountpoint"
        fi
    else
        skip_test "RTK2go connect" "NTRIP_RTK2GO_USER not set"
    fi
    
    # AUSCORS (requires registration, HTTPS)
    if [[ -n "$NTRIP_AUSCORS_USER" && -n "$NTRIP_AUSCORS_PASS" ]]; then
        run_test "AUSCORS connect (ALIC00AUS0)" \
            "$CONNECT" ntrip.data.gnss.ga.gov.au ALIC00AUS0 443 \
            --user="$NTRIP_AUSCORS_USER" --pass="$NTRIP_AUSCORS_PASS" --tls
    else
        skip_test "AUSCORS connect" "NTRIP_AUSCORS_USER/PASS not set"
    fi
    
    # Emlid (if configured)
    if [[ -n "$NTRIP_EMLID_HOST" && -n "$NTRIP_EMLID_MOUNT" ]]; then
        run_test "Emlid connect ($NTRIP_EMLID_MOUNT)" \
            "$CONNECT" "$NTRIP_EMLID_HOST" "$NTRIP_EMLID_MOUNT" "${NTRIP_EMLID_PORT:-2101}" \
            --user="${NTRIP_EMLID_USER:-}" --pass="${NTRIP_EMLID_PASS:-}"
    else
        skip_test "Emlid connect" "NTRIP_EMLID_HOST/MOUNT not set"
    fi
}

# =============================================================================
# STRESS TESTS
# =============================================================================
run_stress_tests() {
    echo -e "\n${BLUE}=== STRESS TESTS ===${NC}"
    
    local cycles="${STRESS_TEST_CYCLES:-5}"
    
    echo -e "Running $cycles rapid sourcetable fetches..."
    local success=0
    for i in $(seq 1 $cycles); do
        if timeout 10 "$SOURCETABLE" rtk2go.com 2101 > /dev/null 2>&1; then
            ((success++))
        fi
        sleep 0.5
    done
    
    if [[ $success -eq $cycles ]]; then
        echo -e "  ${GREEN}PASS${NC}: $success/$cycles successful"
        ((PASS++))
    else
        echo -e "  ${RED}FAIL${NC}: $success/$cycles successful"
        ((FAIL++))
    fi
}

# =============================================================================
# MAIN
# =============================================================================

# Parse arguments
MODE="${1:---quick}"

case "$MODE" in
    --quick)
        run_sourcetable_tests
        ;;
    --full)
        run_sourcetable_tests
        run_nearest_tests
        ;;
    --connect)
        run_sourcetable_tests
        run_nearest_tests
        run_connection_tests
        ;;
    --all)
        run_sourcetable_tests
        run_nearest_tests
        run_connection_tests
        run_stress_tests
        ;;
    --help|-h)
        echo "Usage: $0 [--quick|--full|--connect|--all]"
        echo ""
        echo "  --quick    Sourcetable tests only (default)"
        echo "  --full     Sourcetable + nearest tests"
        echo "  --connect  All tests including connections"
        echo "  --all      All tests including stress tests"
        exit 0
        ;;
    *)
        echo "Unknown option: $MODE"
        echo "Use --help for usage"
        exit 1
        ;;
esac

print_summary
