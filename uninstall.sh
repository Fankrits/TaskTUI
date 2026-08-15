#!/bin/sh
# TaskTUI - Fast, lightweight terminal task manager uninstaller
# Usage: curl -fsSL https://raw.githubusercontent.com/Fankrits/TaskTUI/main/uninstall.sh | sh

set -e

# ANSI color codes
BOLD="$(tput bold 2>/dev/null || printf '')"
GREEN="$(tput setaf 2 2>/dev/null || printf '')"
BLUE="$(tput setaf 4 2>/dev/null || printf '')"
YELLOW="$(tput setaf 3 2>/dev/null || printf '')"
RED="$(tput setaf 1 2>/dev/null || printf '')"
RESET="$(tput sgr0 2>/dev/null || printf '')"

printf "%s\n" "${BLUE}${BOLD}⚡ TaskTUI Uninstaller${RESET}"
printf "%s\n" "==============================="

TARGET_FILES="
$HOME/.local/bin/tasktui
$HOME/.local/bin/task_tui
$HOME/.cargo/bin/tasktui
$HOME/.cargo/bin/task_tui
/usr/local/bin/tasktui
/usr/local/bin/task_tui
"

REMOVED=0

for FILE in $TARGET_FILES; do
  if [ -f "$FILE" ] || [ -L "$FILE" ]; then
    printf "Removing %s...\n" "$FILE"
    if [ -w "$(dirname "$FILE")" ] || [ -w "$FILE" ]; then
      rm -f "$FILE"
      REMOVED=1
    else
      printf "%s\n" "${YELLOW}Elevated permissions required for $FILE. Prompting for sudo:${RESET}"
      sudo rm -f "$FILE"
      REMOVED=1
    fi
  fi
done

if [ "$REMOVED" -eq 1 ]; then
  printf "\n%s\n" "${GREEN}${BOLD}🎉 TaskTUI was successfully uninstalled from your system.${RESET}"
else
  printf "\n%s\n" "${YELLOW}No TaskTUI installations found in standard paths.${RESET}"
fi
