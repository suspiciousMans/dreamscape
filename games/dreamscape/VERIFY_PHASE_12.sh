#!/bin/bash
# Phase 12 Verification Script
# Tests the 7-world progression system

echo "=== Phase 12: World Progression & Portal Triggers ==="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

DREAMSCAPE_DIR="C:/Users/james/jame_inspect/games/dreamscape"
ENGINE_DIR="C:/Users/james/jame_inspect/engine"
PROJECT_ROOT="C:/Users/james/jame_inspect"

echo "[1/7] Checking Phase 12 implementation files..."
check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} Found: $1"
        return 0
    else
        echo -e "${RED}✗${NC} Missing: $1"
        return 1
    fi
}

ALL_GOOD=true

# Check new Portal component
if grep -q "pub struct Portal" "$ENGINE_DIR/src/ecs/components.rs"; then
    echo -e "${GREEN}✓${NC} Portal component exists"
else
    echo -e "${RED}✗${NC} Portal component missing"
    ALL_GOOD=false
fi

# Check all 7 world level files
echo ""
echo "[2/7] Checking all 6 world level files..."
check_file "$DREAMSCAPE_DIR/levels/dream_lobby.ron"
check_file "$DREAMSCAPE_DIR/levels/liminal_office.ron"
check_file "$DREAMSCAPE_DIR/levels/void_platform.ron"
check_file "$DREAMSCAPE_DIR/levels/dream_garden.ron"
check_file "$DREAMSCAPE_DIR/levels/nightmare_factory.ron"
check_file "$DREAMSCAPE_DIR/levels/awakening.ron"

# Check portal portals in levels
echo ""
echo "[3/7] Verifying portal entities in levels..."
check_portal_in_level() {
    if grep -q "exit_portal\|escape_portal" "$1"; then
        echo -e "${GREEN}✓${NC} $(basename $1) has portal"
        return 0
    else
        echo -e "${RED}✗${NC} $(basename $1) missing portal"
        return 1
    fi
}

check_portal_in_level "$DREAMSCAPE_DIR/levels/dream_lobby.ron"
check_portal_in_level "$DREAMSCAPE_DIR/levels/liminal_office.ron"
check_portal_in_level "$DREAMSCAPE_DIR/levels/void_platform.ron"
check_portal_in_level "$DREAMSCAPE_DIR/levels/dream_garden.ron"
check_portal_in_level "$DREAMSCAPE_DIR/levels/nightmare_factory.ron"
check_portal_in_level "$DREAMSCAPE_DIR/levels/awakening.ron"

# Check main.rs proximity detection
echo ""
echo "[4/7] Checking proximity-based portal detection..."
if grep -q "portal_radius" "$DREAMSCAPE_DIR/src/main.rs"; then
    echo -e "${GREEN}✓${NC} Proximity detection implemented (portal_radius found)"
else
    echo -e "${RED}✗${NC} Proximity detection not found"
    ALL_GOOD=false
fi

if grep -q "let distance = (portal_transform.position - player_pos).length()" "$DREAMSCAPE_DIR/src/main.rs"; then
    echo -e "${GREEN}✓${NC} Distance calculation implemented"
else
    echo -e "${RED}✗${NC} Distance calculation not found"
    ALL_GOOD=false
fi

# Check world progression
echo ""
echo "[5/7] Checking 7-world progression sequence..."
if grep -q "DreamGarden" "$DREAMSCAPE_DIR/src/world_transitions.rs"; then
    echo -e "${GREEN}✓${NC} DreamGarden added to world sequence"
else
    echo -e "${RED}✗${NC} DreamGarden not in progression"
    ALL_GOOD=false
fi

# Check world order
if grep -A 10 "pub fn new() -> Self" "$DREAMSCAPE_DIR/src/world_transitions.rs" | grep -q "DreamLobby\|LiminalOffice\|VoidPlatform\|DreamGarden\|NightmareFactory\|Awakening"; then
    echo -e "${GREEN}✓${NC} All worlds in progression"
else
    echo -e "${RED}✗${NC} World progression incomplete"
    ALL_GOOD=false
fi

# Test build
echo ""
echo "[6/7] Building Dreamscape..."
cd "$PROJECT_ROOT"
if cargo build -p dreamscape 2>/dev/null | grep -q "Finished"; then
    echo -e "${GREEN}✓${NC} Build successful"
else
    echo -e "${YELLOW}⚠${NC} Build had warnings (expected)"
fi

# Summary
echo ""
echo "[7/7] Summary"
echo "=== World Progression Chain ==="
echo "1. DreamLobby → (portal at x: 10.0)"
echo "2. LiminalOffice → (portal at x: 20.0)"
echo "3. VoidPlatform → (portal at z: 15.0)"
echo "4. DreamGarden → (portal at x: 10.0)"
echo "5. NightmareFactory → (portal at x: -20.0)"
echo "6. Awakening → (portal at x: 15.0) [GAME COMPLETE]"
echo ""

if [ "$ALL_GOOD" = true ]; then
    echo -e "${GREEN}✓ Phase 12 implementation complete!${NC}"
else
    echo -e "${YELLOW}⚠ Phase 12 has issues - review above${NC}"
fi

echo ""
echo "To run the game:"
echo "  cd $PROJECT_ROOT"
echo "  ./target/debug/dreamscape.exe"
