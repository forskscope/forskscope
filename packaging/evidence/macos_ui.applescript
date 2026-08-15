-- M5-A: macOS UI-scripting driver for the evidence harness (RFC-078).
--
-- Companion to `macos_harness.py`, which shells out to this script via
-- `osascript` for every accessibility query/action. Kept as a standalone
-- AppleScript file (not `osascript -e` one-liners embedded in Python) so
-- the tree-walk/dispatch logic is readable and diffable on its own.
--
-- Why AppleScript + System Events, not a synthetic-input approach: the
-- Linux harness (`linux_harness.py`) spent ~10 CI iterations on X11 input
-- synthesis (XTest, XSendEvent, focus/activate) before switching to
-- AT-SPI's `Action.do_action` - invoking a button's action directly
-- through the accessibility bridge rather than simulating a physical
-- click. System Events' `click`/`perform action "AXPress"` against a
-- `UI element` reference is the same idea on macOS: it drives the
-- accessible action WebKit exposes for the button's `onclick` handler,
-- without needing real window focus, cursor position, or key events.
--
-- Why not pyobjc/ApplicationServices directly: it would need `pip
-- install pyobjc-framework-*` in CI (a new tooling dependency to manage)
-- and its own accessibility-permission grant for the *Python* process,
-- separate from whatever `osascript`/System Events already has. AppleScript
-- via `osascript` is already on every macOS runner with no install step,
-- and CI accessibility recipes (Xcode UI tests, fastlane dialog handling)
-- already rely on System Events being permitted - so it is the path most
-- likely to already work under `macos-latest`'s existing permissions, and
-- the one to try first per the handoff's instruction to prefer direct
-- action invocation over input synthesis.
--
-- ARIA-to-AXRole mapping this relies on (WebKit's accessibility mapping):
--   role="row"      -> AXRole "AXRow"     (hunk.rs's RowLeft/RowRight)
--   button elements -> AXRole "AXButton", with the `aria_label` exposed
--                       as AXDescription (title is often empty for a
--                       WebKit-rendered button, so both are checked)
--   plain text (<p>) -> exposed via `value`/`description` of a descendant
--                       static-text element, so both are checked too.
--
-- Usage: osascript macos_ui.applescript <command> <process-name> [<needle>]
--   window_size <proc>            -> "WxH" of window 1, or NO_PROCESS/NO_WINDOW
--   count_rows <proc>             -> integer count of AXRow elements anywhere
--                                     in window 1's accessible tree
--   find_text <proc> <needle>     -> "FOUND: <text>" or "NOT_FOUND"
--   click_button <proc> <needle>  -> "CLICKED: <desc>", "NOT_FOUND", or
--                                     "DISABLED" (found but not clickable -
--                                     not clicked, so caller can retry)
--
-- Every command returns a plain string on stdout; a System Events
-- permission failure (e.g. "not allowed assistive access") surfaces as an
-- AppleScript runtime error on stderr with non-zero exit, which the
-- Python side treats as its own distinct, reportable failure mode rather
-- than papering over it as "element not found".

on run argv
    set cmdName to item 1 of argv
    set procName to item 2 of argv

    tell application "System Events"
        if not (exists process procName) then
            return "NO_PROCESS"
        end if
        tell process procName
            if (count of windows) = 0 then
                return "NO_WINDOW"
            end if
            set w to window 1

            if cmdName is "window_size" then
                set sz to size of w
                return ((item 1 of sz) as string) & "x" & ((item 2 of sz) as string)

            else if cmdName is "count_rows" then
                set n to 0
                set allEl to entire contents of w
                repeat with e in allEl
                    try
                        if role of e is "AXRow" then set n to n + 1
                    end try
                end repeat
                return n as string

            else if cmdName is "find_text" then
                set needle to item 3 of argv
                set allEl to entire contents of w
                repeat with e in allEl
                    try
                        set d to description of e
                        if d contains needle then return "FOUND: " & d
                    end try
                    try
                        set t to title of e
                        if t contains needle then return "FOUND: " & t
                    end try
                    try
                        set v to value of e
                        if class of v is text then
                            if v contains needle then return "FOUND: " & v
                        end if
                    end try
                end repeat
                return "NOT_FOUND"

            else if cmdName is "click_button" then
                set needle to item 3 of argv
                set allEl to entire contents of w
                repeat with e in allEl
                    set isButton to false
                    try
                        if role of e is "AXButton" then set isButton to true
                    end try
                    if isButton then
                        set d to ""
                        try
                            set d to description of e
                        end try
                        if d is "" then
                            try
                                set d to title of e
                            end try
                        end if
                        if d contains needle then
                            set isEnabled to true
                            try
                                set isEnabled to (enabled of e)
                            end try
                            if isEnabled then
                                click e
                                return "CLICKED: " & d
                            else
                                return "DISABLED: " & d
                            end if
                        end if
                    end if
                end repeat
                return "NOT_FOUND"

            else
                return "UNKNOWN_COMMAND: " & cmdName
            end if
        end tell
    end tell
end run
