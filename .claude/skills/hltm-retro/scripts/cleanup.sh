#!/bin/bash
# HLTM Cleanup Script
# Run ONLY after Cleanup Gate passes

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}=== HLTM CLEANUP ===${NC}"

# Safety check: must be in project root
if [ ! -f ".hltm/session.yaml" ]; then
    echo -e "${RED}ERROR: .hltm/session.yaml not found. Run from project root.${NC}"
    exit 1
fi

# Safety check: confirm
echo -e "${YELLOW}This will DELETE:${NC}"
echo "  - docs/epics/*"
echo "  - docs/stories/*"
echo "  - docs/prd.md"
echo "  - sprint-status.yaml"
echo ""
echo -e "${YELLOW}This will ARCHIVE:${NC}"
echo "  - input/brief.md → input/completed/brief-YYYY-MM-DD-HH-MM.md"
echo "  - input/update.md → input/completed/update-YYYY-MM-DD-HH-MM.md"
echo ""
read -p "Continue? [y/N] " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 0
fi

# Delete epics
if [ -d "docs/epics" ]; then
    rm -rf docs/epics
    echo -e "${GREEN}✓ Deleted docs/epics/${NC}"
else
    echo -e "${YELLOW}⊘ docs/epics/ not found${NC}"
fi

# Delete stories (if separate folder)
if [ -d "docs/stories" ]; then
    rm -rf docs/stories
    echo -e "${GREEN}✓ Deleted docs/stories/${NC}"
fi

# Delete PRD
if [ -f "docs/prd.md" ]; then
    rm docs/prd.md
    echo -e "${GREEN}✓ Deleted docs/prd.md${NC}"
else
    echo -e "${YELLOW}⊘ docs/prd.md not found${NC}"
fi

# Delete sprint-status
if [ -f "sprint-status.yaml" ]; then
    rm sprint-status.yaml
    echo -e "${GREEN}✓ Deleted sprint-status.yaml${NC}"
else
    echo -e "${YELLOW}⊘ sprint-status.yaml not found${NC}"
fi

# Archive input files
TIMESTAMP=$(date +%Y-%m-%d-%H-%M)

if [ -f "input/brief.md" ] || [ -f "input/update.md" ]; then
    mkdir -p input/completed

    if [ -f "input/brief.md" ]; then
        mv input/brief.md "input/completed/brief-${TIMESTAMP}.md"
        echo -e "${GREEN}✓ Archived input/brief.md → input/completed/brief-${TIMESTAMP}.md${NC}"
    fi

    if [ -f "input/update.md" ]; then
        mv input/update.md "input/completed/update-${TIMESTAMP}.md"
        echo -e "${GREEN}✓ Archived input/update.md → input/completed/update-${TIMESTAMP}.md${NC}"
    fi
else
    echo -e "${YELLOW}⊘ No input files to archive${NC}"
fi

# Update session to idle
if command -v yq &> /dev/null; then
    yq -i '.phase = "idle"' .hltm/session.yaml
    yq -i 'del(.epic)' .hltm/session.yaml
    yq -i 'del(.story)' .hltm/session.yaml
    yq -i 'del(.step)' .hltm/session.yaml
    yq -i '.attempt = 0' .hltm/session.yaml
    yq -i '.recovery_used = false' .hltm/session.yaml
    echo -e "${GREEN}✓ Reset .hltm/session.yaml to idle${NC}"
else
    echo -e "${YELLOW}⚠ yq not installed. Update .hltm/session.yaml manually:${NC}"
    echo "  phase: idle"
fi

# Verify
echo ""
echo -e "${GREEN}=== POST-CLEANUP CHECK ===${NC}"

checks_passed=true

# Check required files exist
for file in "docs/project-context.md" "docs/architecture.md"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓ $file exists${NC}"
    else
        echo -e "${RED}✗ $file MISSING${NC}"
        checks_passed=false
    fi
done

# Check deleted files gone
for file in "docs/prd.md" "sprint-status.yaml"; do
    if [ ! -f "$file" ]; then
        echo -e "${GREEN}✓ $file deleted${NC}"
    else
        echo -e "${RED}✗ $file still exists${NC}"
        checks_passed=false
    fi
done

# Check folders empty/gone
for dir in "docs/epics" "docs/stories"; do
    if [ ! -d "$dir" ] || [ -z "$(ls -A $dir 2>/dev/null)" ]; then
        echo -e "${GREEN}✓ $dir empty/gone${NC}"
    else
        echo -e "${RED}✗ $dir not empty${NC}"
        checks_passed=false
    fi
done

echo ""
if [ "$checks_passed" = true ]; then
    echo -e "${GREEN}=== CLEANUP COMPLETE ===${NC}"
else
    echo -e "${RED}=== CLEANUP INCOMPLETE - CHECK ERRORS ===${NC}"
    exit 1
fi
