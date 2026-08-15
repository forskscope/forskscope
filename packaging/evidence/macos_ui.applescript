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
-- M5-B additions (P04/P05/P06/P08/P12 - all need in-app interaction beyond
-- what M5-A's four commands above cover: rows that are clickable but are
-- not AXButton, popup/combo controls (Theme/Language/font-family selects),
-- a numeric spinner (font size), and reading back a control's own value
-- rather than only finding text on the page):
--   click_row <proc> <needle>      -> "CLICKED: <label>", "NOT_FOUND" - like
--                                      click_button but for role "AXRow"
--                                      (Explorer's TreeRow, hunk.rs's
--                                      RowLeft/RowRight are role="row" too,
--                                      so `needle` should be specific enough
--                                      to disambiguate, e.g. a file name)
--   get_value <proc> <role> <n>    -> "VALUE: <v>", "NOT_FOUND" - the AXValue
--                                      of the n-th (1-based) element with the
--                                      given AXRole, in tree-walk order
--   set_value <proc> <role> <n> <v> -> "SET: <before> -> <after>",
--                                      "NOT_FOUND", or "ERROR: <msg>" - sets
--                                      AXValue directly (the same kind of
--                                      direct accessibility write click_button
--                                      already relies on for AXPress, applied
--                                      to value instead of action - not
--                                      synthetic keystrokes). `<after>` is
--                                      read back post-write so the caller can
--                                      detect a write that silently no-opped.
--   perform_action <proc> <role> <n> <action>
--                                   -> "DONE", "NOT_FOUND", or "ERROR: <msg>"
--                                      - invokes a named AXAction (e.g.
--                                      "AXIncrement") on the n-th element of
--                                      the given role, for controls where a
--                                      direct AXValue write may not be
--                                      honoured (spinner fallback).
--   close_window <proc>            -> "CLOSED" or "ERROR: <msg>" - clicks the
--                                      window's own close (traffic-light)
--                                      button, for a normal-quit test distinct
--                                      from `terminate()`'s SIGTERM (P12 needs
--                                      a real "quit and flush session" path,
--                                      not a signal).
--   dump_roles <proc> [<cap>]      -> one "role|title|description|value|enabled"
--                                      line per element (default cap 300) -
--                                      a debugging aid for recording exactly
--                                      what WebKit's accessibility mapping
--                                      produces for a given view, not used by
--                                      any case's pass/fail logic itself.
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

            else if cmdName is "click_row" then
                set needle to item 3 of argv
                set allEl to entire contents of w
                repeat with e in allEl
                    set isRow to false
                    try
                        if role of e is "AXRow" then set isRow to true
                    end try
                    if isRow then
                        set hit to false
                        set d to ""
                        try
                            if (description of e) contains needle then
                                set hit to true
                                set d to description of e
                            end if
                        end try
                        if not hit then
                            try
                                set innerEl to entire contents of e
                                repeat with ie in innerEl
                                    try
                                        set iv to value of ie
                                        if class of iv is text and iv contains needle then
                                            set hit to true
                                            set d to iv
                                            exit repeat
                                        end if
                                    end try
                                end repeat
                            end try
                        end if
                        if hit then
                            click e
                            return "CLICKED: " & d
                        end if
                    end if
                end repeat
                return "NOT_FOUND"

            else if cmdName is "get_value" then
                set roleWanted to item 3 of argv
                set n to (item 4 of argv) as integer
                set idx to 0
                set allEl to entire contents of w
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is roleWanted then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            try
                                return "VALUE: " & (value of e)
                            on error errMsg
                                return "ERROR: " & errMsg
                            end try
                        end if
                    end if
                end repeat
                return "NOT_FOUND"

            else if cmdName is "set_value" then
                set roleWanted to item 3 of argv
                set n to (item 4 of argv) as integer
                set newVal to item 5 of argv
                set idx to 0
                set allEl to entire contents of w
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is roleWanted then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            set beforeVal to ""
                            try
                                set beforeVal to (value of e) as string
                            end try
                            try
                                set value of e to newVal
                            on error errMsg
                                return "ERROR: " & errMsg
                            end try
                            set afterVal to ""
                            try
                                set afterVal to (value of e) as string
                            end try
                            return "SET: " & beforeVal & " -> " & afterVal
                        end if
                    end if
                end repeat
                return "NOT_FOUND"

            else if cmdName is "perform_action" then
                set roleWanted to item 3 of argv
                set n to (item 4 of argv) as integer
                set actionName to item 5 of argv
                set idx to 0
                set allEl to entire contents of w
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is roleWanted then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            try
                                perform action actionName of e
                            on error errMsg
                                return "ERROR: " & errMsg
                            end try
                            return "DONE"
                        end if
                    end if
                end repeat
                return "NOT_FOUND"

            else if cmdName is "close_window" then
                try
                    click button 1 of w
                    return "CLOSED"
                on error errMsg
                    return "ERROR: " & errMsg
                end try

            else if cmdName is "dump_roles" then
                set capN to 300
                if (count of argv) > 2 then set capN to (item 3 of argv) as integer
                set outLines to {}
                set allEl to entire contents of w
                set seen to 0
                repeat with e in allEl
                    set seen to seen + 1
                    if seen > capN then exit repeat
                    set rl to ""
                    set ti to ""
                    set de to ""
                    set va to ""
                    set en to ""
                    try
                        set rl to (role of e) as string
                    end try
                    try
                        set ti to (title of e) as string
                    end try
                    try
                        set de to (description of e) as string
                    end try
                    try
                        set va to (value of e) as string
                    end try
                    try
                        set en to (enabled of e) as string
                    end try
                    set end of outLines to rl & "|" & ti & "|" & de & "|" & va & "|" & en
                end repeat
                set AppleScript's text item delimiters to linefeed
                set outStr to outLines as string
                set AppleScript's text item delimiters to ""
                return outStr

            else
                return "UNKNOWN_COMMAND: " & cmdName
            end if
        end tell
    end tell
end run
