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
--   click_button_exact <proc> <needle> -> like click_button, but exact
--                                      match (`d is needle`) instead of
--                                      substring - for button-text pairs
--                                      that collide under `contains`
--                                      (e.g. Save As modal's plain "Save"
--                                      vs the toolbar's "Save merge result
--                                      (Ctrl+S)").
--   click_row <proc> <needle>      -> "CLICKED: <label>", "NOT_FOUND" - like
--                                      click_button but for role "AXRow"
--                                      (Explorer's TreeRow, hunk.rs's
--                                      RowLeft/RowRight are role="row" too,
--                                      so `needle` should be specific enough
--                                      to disambiguate, e.g. a file name)
--   click_row_side <proc> <needle> <left|right>
--                                   -> "CLICKED: x=<n>", "NOT_FOUND", or
--                                      "ERROR: <msg>" - Explorer's Aligned
--                                      view shows the same directory in
--                                      both panes by default, so a
--                                      filename can appear as an AXRow on
--                                      both sides; click_row's first-match
--                                      behaviour can click the wrong
--                                      pane's copy. Picks the matching row
--                                      with the min ("left") or max
--                                      ("right") X position instead.
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
--   select_popup_item <proc> <n> <label>
--                                   -> "SELECTED: <label>", "NOT_FOUND", or
--                                      "ERROR: <msg>" - set_value/
--                                      perform_action's AXIncrement both
--                                      confirmed silent no-ops against
--                                      Theme/Language/font-family's
--                                      AXPopUpButton controls (recon:
--                                      "SET: Dark -> Dark"). Clicks the n-th
--                                      AXPopUpButton (opens its menu, a real
--                                      click) then clicks the named item in
--                                      that menu - real menu-item selection.
--   type_into <proc> <role> <n> <text>
--                                   -> "TYPED: <value-read-back>",
--                                      "NOT_FOUND", or "ERROR: <msg>" - same
--                                      no-op problem for the font-size
--                                      spinner's AXValue. Focuses the n-th
--                                      element of the given role, selects all
--                                      (Cmd+A) and types `text` via real
--                                      `keystroke` - see the command's own
--                                      comment for why this is a different
--                                      situation from the Linux harness's
--                                      keystroke-synthesis problems.
--   close_window <proc>            -> "CLOSED" or "ERROR: <msg>" - clicks the
--                                      window's own close (traffic-light)
--                                      button, for a normal-quit test distinct
--                                      from `terminate()`'s SIGTERM (P12 needs
--                                      a real "quit and flush session" path,
--                                      not a signal).
--   dump_roles <proc> [<cap>]      -> one "role|title|description|value"
--                                      line per element (default cap 300) -
--                                      a debugging aid for recording exactly
--                                      what WebKit's accessibility mapping
--                                      produces for a given view, not used by
--                                      any case's pass/fail logic itself.
--
-- M5-C additions (F63 investigation, P03's scroll-mirroring check):
--   list_roles <proc>              -> "AXRow=N AXScrollArea=N AXScrollBar=N
--                                      AXStaticText=N AXGroup=N AXWebArea=N
--                                      total=N" - a bounded tally of a FIXED
--                                      small set of roles of interest (not
--                                      dump_roles' abandoned open-ended bulk
--                                      dump - see that command's comment),
--                                      built to answer one question: does a
--                                      given view expose a real
--                                      AXScrollArea/AXScrollBar distinct
--                                      from "how many AXRow currently
--                                      exist" (which count_rows answers).
--   send_key <proc> <keycode> <n>  -> "DONE" or "ERROR: ..." - activates
--                                      the process then sends the given key
--                                      code n times with a short delay
--                                      between presses (real key events, via
--                                      `key code`, not `keystroke` - this is
--                                      navigation input, not text). Used to
--                                      test whether keyboard-driven scrolling
--                                      changes what the accessibility tree
--                                      exposes (F63) and, for P03, to drive
--                                      one pane's horizontal scroll and
--                                      check the other pane follows.
--   double_click_row_side <proc> <needle> <left|right>
--                                   -> "CLICKED: occurrence=N", "NOT_FOUND",
--                                      or "ERROR: ..." - like
--                                      click_row_side, but two real
--                                      `click at {x,y}` mouse events at the
--                                      row's own screen position (no
--                                      AXDoublePress equivalent exists) to
--                                      trigger Explorer's directory-navigate
--                                      double-click handler, for P07's
--                                      navigation/history check.
--   focused_element <proc>         -> "FOCUSED: source=<process|window>
--                                      class=<c> role=<r> desc=<d>
--                                      title=<t> value=<v>", "FOCUSED:
--                                      MISSING ...", or "ERROR: ..." - reads
--                                      back which element currently holds
--                                      keyboard focus via the aggregate
--                                      AXFocusedUIElement pointer (process-
--                                      level, falling back to window-level),
--                                      with no input synthesized at all.
--   find_focused <proc> <role>     -> "FOCUSED: <desc>" or "NONE_FOCUSED" -
--                                      per-element AXFocused boolean walk
--                                      over every element of the given role
--                                      (e.g. "AXButton") - the technique
--                                      that actually works for P11's modal-
--                                      focus-on-safe-action check, since
--                                      focused_element's aggregate pointer
--                                      resolved to `missing value` for this
--                                      WKWebView-hosted content on real
--                                      dispatch.
--
-- Every command returns a plain string on stdout; a System Events
-- permission failure (e.g. "not allowed assistive access") surfaces as an
-- AppleScript runtime error on stderr with non-zero exit, which the
-- Python side treats as its own distinct, reportable failure mode rather
-- than papering over it as "element not found".

-- M5-B: recon against the Explorer view found `entire contents of w`
-- itself throwing "System Events got an error: AppleEvent handler failed.
-- (-10000)" - deterministically, not as a transient race (M5-A's four
-- cases never hit this because none of them target the Explorer view;
-- their `entire contents of w` calls are left exactly as they were).
-- `entire contents` is one bulk AppleEvent that resolves the *entire*
-- subtree at once; some node under Explorer's WKWebView-rendered tree
-- (likely the async-scanning file tree itself - `tree.rs`'s scans_l/
-- scans_r channels mutate it while System Events is trying to resolve a
-- stable snapshot) makes that single bulk call fail as a whole, with no
-- way to skip just the poison node. `safeContents` falls back to a manual,
-- per-node `UI elements of` walk (`flatWalk`) that tolerates any single
-- node refusing to enumerate its children - it just treats that node as a
-- leaf instead of failing the entire traversal. The bounded retry loop
-- below stays as a second, independent layer - even a per-node walk can
-- still race a concurrent mutation at the exact node it is standing on.
on run argv
    set attemptN to 0
    repeat
        set attemptN to attemptN + 1
        try
            return runOnce(argv)
        on error errMsg number errNum
            if errNum is -10000 and attemptN < 6 then
                delay 0.5
            else
                error ("after " & attemptN & " attempt(s): " & errMsg) number errNum
            end if
        end try
    end repeat
end run

on runOnce(argv)
    set cmdName to item 1 of argv
    set procName to item 2 of argv

    tell application "System Events"
        -- M5-B P06 (two-process variant): when a second same-named
        -- "forskscope" process launches while a first one is still being
        -- torn down, `process procName` addressing can resolve to
        -- whichever the window server happens to enumerate first - which
        -- was observed to be the dying one (count_rows returned 0, only a
        -- single stale text fragment left, even though the OS process had
        -- already been reaped per `proc.poll()`). A caller can pass
        -- "pid:<n>" instead of a plain name to address a process
        -- unambiguously by PID, sidestepping the name collision entirely.
        if procName starts with "pid:" then
            set targetPid to (text 5 thru -1 of procName) as integer
            set matchingProcs to (every process whose unix id is targetPid)
            if (count of matchingProcs) is 0 then
                return "NO_PROCESS"
            end if
            set targetProc to item 1 of matchingProcs
        else
            if not (exists process procName) then
                return "NO_PROCESS"
            end if
            set targetProc to process procName
        end if
        tell targetProc
            if (count of windows) = 0 then
                return "NO_WINDOW"
            end if
            set w to window 1

            if cmdName is "window_size" then
                set sz to size of w
                return ((item 1 of sz) as string) & "x" & ((item 2 of sz) as string)

            else if cmdName is "count_rows" then
                set n to 0
                set allEl to my safeContents(w)
                repeat with e in allEl
                    try
                        if role of e is "AXRow" then set n to n + 1
                    end try
                end repeat
                return n as string

            else if cmdName is "find_text" then
                set needle to item 3 of argv
                set allEl to my safeContents(w)
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
                set allEl to my safeContents(w)
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

            else if cmdName is "click_button_exact" then
                -- Same as click_button, but `d is needle` (exact match)
                -- instead of `d contains needle` - some button pairs
                -- collide under substring matching (e.g. a Save As
                -- modal's plain "Save" vs the toolbar's "Save merge
                -- result (Ctrl+S)", which contains "Save" too).
                set needle to item 3 of argv
                set allEl to my safeContents(w)
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
                        if d is needle then
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
                set allEl to my safeContents(w)
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
                                set innerEl to my safeContents(e)
                                repeat with ie in innerEl
                                    try
                                        set iv to value of ie
                                        if class of iv is text and iv contains needle then
                                            set hit to true
                                            set d to iv
                                            exit repeat
                                        end if
                                    end try
                                    if not hit then
                                        try
                                            set it2 to title of ie
                                            if it2 contains needle then
                                                set hit to true
                                                set d to it2
                                                exit repeat
                                            end if
                                        end try
                                    end if
                                    if not hit then
                                        try
                                            set id2 to description of ie
                                            if id2 contains needle then
                                                set hit to true
                                                set d to id2
                                                exit repeat
                                            end if
                                        end try
                                    end if
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

            else if cmdName is "click_row_side" then
                -- Explorer's Aligned view (tree.rs) shows the SAME
                -- directory in both panes by default (default_explorer_dir
                -- with remember_explorer_dirs off), so a given filename
                -- appears as an AXRow on *both* sides - click_row's
                -- first-match behaviour picked the wrong pane's copy in
                -- P06 recon (both clicks ended up setting left_pick,
                -- Compare stayed disabled).
                --
                -- First fix attempt queried `position of e` per candidate
                -- to disambiguate by X coordinate - that made click_row_side
                -- itself time out (45s+) even after switching its inner
                -- text search to direct children only and shrinking the
                -- fixtures to 4,000 lines, while click_any (no position
                -- query) stayed fast. The same shape as dump_roles'
                -- `enabled of e` dead end earlier in this investigation: an
                -- innocuous-looking property query that is disproportionately
                -- expensive (or outright wedges something) for this
                -- WebKit-rendered element class. Avoided entirely instead
                -- of chased further.
                --
                -- tree.rs's aligned-row markup renders left's `pane-half`
                -- before right's for the same row (`div.pane-half { left
                -- TreeRow } div.pane-half { right TreeRow }`), and a
                -- filename that exists in both panes (the aligned view
                -- browsing one shared directory) produces exactly one
                -- aligned-row containing exactly two AXRow matches for
                -- that name - the first in document order is always the
                -- left copy, the second always the right. No coordinates
                -- needed: "left" -> 1st match, "right" -> 2nd match.
                set needle to item 3 of argv
                set side to item 4 of argv
                set wantNth to 1
                if side is "right" then set wantNth to 2
                set allEl to my safeContents(w)
                set seenN to 0
                repeat with e in allEl
                    set isRow to false
                    try
                        if role of e is "AXRow" then set isRow to true
                    end try
                    if isRow then
                        set hit to false
                        -- Direct children only (`UI elements of e`), not a
                        -- full recursive safeContents(e) - TreeRow's own
                        -- markup (dir_pane.rs) is shallow (caret/icon/
                        -- label/status spans, no nesting), so this is both
                        -- correct and far cheaper per row.
                        try
                            set innerEl to (UI elements of e)
                            repeat with ie in innerEl
                                try
                                    set iv to value of ie
                                    if class of iv is text and iv contains needle then
                                        set hit to true
                                        exit repeat
                                    end if
                                end try
                                if not hit then
                                    try
                                        set it3 to title of ie
                                        if it3 contains needle then
                                            set hit to true
                                            exit repeat
                                        end if
                                    end try
                                end if
                            end repeat
                        end try
                        if hit then
                            set seenN to seenN + 1
                            if seenN is wantNth then
                                try
                                    click e
                                    return "CLICKED: occurrence=" & seenN
                                on error errMsg
                                    return "ERROR: click: " & errMsg
                                end try
                            end if
                        end if
                    end if
                end repeat
                return "NOT_FOUND"

            else if cmdName is "double_click_row_side" then
                -- M5-C / P07's navigation-history check: `click_row_side`
                -- (above) invokes AXPress via `click e`, which fires the
                -- row's real onclick handler (on_select - picks the row,
                -- does not navigate into it). Navigating INTO a directory
                -- is tree.rs's separate `on_dblclick` handler, which has no
                -- accessible-action equivalent to invoke directly (unlike a
                -- plain click, there is no "AXDoublePress"). The standard
                -- System Events technique for a real double-click is two
                -- genuine `click at {x,y}` mouse events at the element's
                -- own screen position, close enough together to register as
                -- a double-click at the OS level (the same position-based
                -- click `type_into` already established as reliable for
                -- delivering real input this harness's element-reference
                -- `click e` cannot). Reuses click_row_side's left/right
                -- occurrence disambiguation (1st match = left pane's copy,
                -- 2nd = right's) since Explorer's aligned view shows the
                -- same directory names on both sides by default.
                set needle to item 3 of argv
                set side to item 4 of argv
                set wantNth to 1
                if side is "right" then set wantNth to 2
                set allEl to my safeContents(w)
                set seenN to 0
                set target to missing value
                repeat with e in allEl
                    set isRow to false
                    try
                        if role of e is "AXRow" then set isRow to true
                    end try
                    if isRow then
                        set hit to false
                        try
                            set innerEl to (UI elements of e)
                            repeat with ie in innerEl
                                try
                                    set iv to value of ie
                                    if class of iv is text and iv contains needle then
                                        set hit to true
                                        exit repeat
                                    end if
                                end try
                                if not hit then
                                    try
                                        set it5 to title of ie
                                        if it5 contains needle then
                                            set hit to true
                                            exit repeat
                                        end if
                                    end try
                                end if
                            end repeat
                        end try
                        if hit then
                            set seenN to seenN + 1
                            if seenN is wantNth then
                                set target to e
                                exit repeat
                            end if
                        end if
                    end if
                end repeat
                if target is missing value then return "NOT_FOUND"
                try
                    set p to position of target
                    set sz to size of target
                    set cx to (item 1 of p) + ((item 1 of sz) / 2)
                    set cy to (item 2 of p) + ((item 2 of sz) / 2)
                on error errMsg
                    return "ERROR: position/size: " & errMsg
                end try
                try
                    set frontmost of process procName to true
                end try
                delay 0.2
                try
                    click at {cx, cy}
                    delay 0.15
                    click at {cx, cy}
                on error errMsg
                    return "ERROR: double-click-at-coords: " & errMsg
                end try
                return "CLICKED: occurrence=" & seenN & " at={" & cx & "," & cy & "}"

            else if cmdName is "find_focused" then
                -- M5-C / P11's modal-focus check, attempt 2: `focused_element`
                -- (below) queries the aggregate `AXFocusedUIElement` pointer
                -- at the process/window level - two real dispatches showed
                -- that resolves to `missing value` at the process level and
                -- errors outright at the window level for this app, on this
                -- WKWebView-hosted content. Standard NSAccessibility also
                -- exposes focus PER-ELEMENT, as a plain boolean `AXFocused`
                -- attribute (`focused of e` in System Events terms) - this
                -- walks the given role's matching elements (typically
                -- "AXButton" for a modal's actions) and returns whichever
                -- one reports `focused of e = true`, without relying on any
                -- aggregate app/window-level pointer at all.
                set roleWanted to item 3 of argv
                set allEl to my safeContents(w)
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is roleWanted then set isMatch to true
                    end try
                    if isMatch then
                        set isFocused to false
                        try
                            set isFocused to (focused of e)
                        end try
                        if isFocused then
                            set d to ""
                            try
                                set d to description of e
                            end try
                            if d is "" then
                                try
                                    set d to title of e
                                end try
                            end if
                            return "FOCUSED: " & d
                        end if
                    end if
                end repeat
                return "NONE_FOCUSED"

            else if cmdName is "focused_element" then
                -- M5-C / P11's modal-focus check: reads back WHICH element
                -- currently holds keyboard focus, via the process's own
                -- AXFocusedUIElement attribute - a pure read, synthesizes no
                -- input at all (distinct from send_key/type_into/click,
                -- which all perform an action). This is what makes P11's
                -- "modal focus starts on the safe/cancel action" item
                -- CI-verifiable in the first place (handoff M5-C §6): focus
                -- POSITION is exposed through the accessibility tree
                -- whether or not any input is ever synthesized.
                -- Real dispatch #1 found every property empty with no error
                -- (bare `try` swallowed the cause); real dispatch #2, with
                -- per-property error text added, showed `fe` itself was
                -- `missing value` - the process-level AXFocusedUIElement
                -- attribute resolved cleanly but held nothing. Standard
                -- AppleScript UI-scripting fallback for exactly this: focus
                -- is also exposed as an attribute of the frontmost WINDOW,
                -- which is sometimes populated when the application-level
                -- one is not (observed in other AppleScript accessibility
                -- recipes for WebView-hosted content specifically). Try
                -- process-level first, then window-level, and report which
                -- source actually produced a non-missing value.
                set fe to missing value
                set feSource to "none"
                try
                    set fe to (value of attribute "AXFocusedUIElement" of targetProc)
                    if fe is not missing value then set feSource to "process"
                on error errMsg
                    return "ERROR: get-focused-process: " & errMsg
                end try
                if fe is missing value then
                    try
                        set fe to (value of attribute "AXFocusedUIElement" of w)
                        if fe is not missing value then set feSource to "window"
                    on error errMsg2
                        return "ERROR: get-focused-window: " & errMsg2
                    end try
                end if
                if fe is missing value then
                    return "FOCUSED: MISSING (checked process and window attributes, both missing value)"
                end if
                set fclass to "?"
                try
                    set fclass to (class of fe) as string
                on error e0
                    set fclass to "ERR:" & e0
                end try
                set frole to "?"
                try
                    set frole to (role of fe) as string
                on error e1
                    set frole to "ERR:" & e1
                end try
                set fdesc to "?"
                try
                    set fdesc to (description of fe) as string
                on error e2
                    set fdesc to "ERR:" & e2
                end try
                set ftitle to "?"
                try
                    set ftitle to (title of fe) as string
                on error e3
                    set ftitle to "ERR:" & e3
                end try
                set fvalue to "?"
                try
                    set fvalue to (value of fe) as string
                on error e4
                    set fvalue to "ERR:" & e4
                end try
                return "FOCUSED: source=" & feSource & " class=" & fclass & " role=" & frole & " desc=" & fdesc & " title=" & ftitle & " value=" & fvalue

            else if cmdName is "click_any" then
                -- Broadest of the click_* family: no role filter at all.
                -- TabBar's Explorer tab (tabs.rs) is a plain `div` with an
                -- onclick and no `role` attribute set, so it's neither
                -- AXButton (click_button) nor AXRow (click_row) - matches
                -- by the same own-or-nested-text rule as click_row, on any
                -- role, and clicks the first hit in document order.
                set needle to item 3 of argv
                set allEl to my safeContents(w)
                repeat with e in allEl
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
                            set innerEl to my safeContents(e)
                            repeat with ie in innerEl
                                try
                                    set iv to value of ie
                                    if class of iv is text and iv contains needle then
                                        set hit to true
                                        set d to iv
                                        exit repeat
                                    end if
                                end try
                                if not hit then
                                    try
                                        set it4 to title of ie
                                        if it4 contains needle then
                                            set hit to true
                                            set d to it4
                                            exit repeat
                                        end if
                                    end try
                                end if
                            end repeat
                        end try
                    end if
                    if hit then
                        try
                            click e
                            return "CLICKED: " & d
                        on error errMsg
                            -- Not every matched element is itself
                            -- clickable (a static-text leaf inside a
                            -- clickable ancestor, say) - keep looking
                            -- rather than failing outright.
                        end try
                    end if
                end repeat
                return "NOT_FOUND"

            else if cmdName is "get_value" then
                set roleWanted to item 3 of argv
                set n to (item 4 of argv) as integer
                set idx to 0
                set allEl to my safeContents(w)
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
                set allEl to my safeContents(w)
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
                set allEl to my safeContents(w)
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

            else if cmdName is "probe_popup" then
                -- Diagnostic only (not used by any case): select_popup_item
                -- found "Can't get menu 1 of pop up button N ... Invalid
                -- index" - no native AXMenu child at all. Before concluding
                -- these controls have no accessible open-dropdown state
                -- whatsoever, check whether WebKit exposes the open
                -- dropdown's options as ordinary elements elsewhere instead
                -- of via the `menu`/`menu item` collection: how many
                -- windows exist post-click (a floating panel might be a
                -- second window), and how many AXMenuItem/AXRow-role
                -- elements now exist inside window 1 itself.
                set n to (item 3 of argv) as integer
                set idx to 0
                set allEl to my safeContents(w)
                set target to missing value
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is "AXPopUpButton" then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            set target to e
                            exit repeat
                        end if
                    end if
                end repeat
                if target is missing value then return "NOT_FOUND"
                try
                    click target
                on error errMsg
                    return "ERROR: click popup: " & errMsg
                end try
                delay 0.4
                set winCount to 0
                try
                    set winCount to count of windows
                end try
                set miCount to 0
                set rowCount to 0
                set sample to ""
                try
                    set allEl2 to my safeContents(w)
                    repeat with e2 in allEl2
                        set rl2 to ""
                        try
                            set rl2 to role of e2
                        end try
                        if rl2 is "AXMenuItem" then
                            set miCount to miCount + 1
                            try
                                set sample to sample & "[" & (title of e2) & "/" & (description of e2) & "]"
                            end try
                        else if rl2 is "AXRow" then
                            set rowCount to rowCount + 1
                        end if
                    end repeat
                end try
                key code 53 -- Escape, to close whatever may have opened
                return "windows=" & winCount & " menuitems=" & miCount & " rows=" & rowCount & " sample=" & sample

            else if cmdName is "select_popup_item" then
                -- M5-B recon found `set_value`/`perform_action "AXIncrement"`
                -- are silent no-ops against these WebKit-rendered `<select>`
                -- controls (AXValue reads back unchanged either way) - the
                -- direct-write path M5-A's button work generalised from
                -- doesn't hold for this control kind. This is the standard
                -- AppleScript technique for a real popup button instead:
                -- click it (opens its menu, same as a real mouse click
                -- would), then click the wanted item in that menu - actual
                -- menu-item selection, not a value write.
                set n to (item 3 of argv) as integer
                set optionLabel to item 4 of argv
                set idx to 0
                set allEl to my safeContents(w)
                set target to missing value
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is "AXPopUpButton" then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            set target to e
                            exit repeat
                        end if
                    end if
                end repeat
                if target is missing value then return "NOT_FOUND"
                try
                    click target
                on error errMsg
                    return "ERROR: click popup: " & errMsg
                end try
                delay 0.3
                -- probe_popup found the open dropdown's options as plain
                -- AXMenuItem elements inside window 1's own tree (title
                -- holding the option label), NOT nested under a `menu 1 of`
                -- collection the standard `menu item X of menu 1 of Y`
                -- reference syntax expects ("Can't get menu 1 of pop up
                -- button N ... Invalid index") - this control isn't a true
                -- native NSPopUpButton, just styled to look like one. Find
                -- and click the AXMenuItem directly, same pattern as
                -- click_row/click_button.
                set itemEl to missing value
                try
                    set allEl2 to my safeContents(w)
                    repeat with e2 in allEl2
                        set rl2 to ""
                        try
                            set rl2 to role of e2
                        end try
                        if rl2 is "AXMenuItem" then
                            set lbl to ""
                            try
                                set lbl to title of e2
                            end try
                            if lbl is "" then
                                try
                                    set lbl to description of e2
                                end try
                            end if
                            if lbl is optionLabel then
                                set itemEl to e2
                                exit repeat
                            end if
                        end if
                    end repeat
                end try
                if itemEl is missing value then
                    try
                        key code 53 -- Escape, don't leave the dropdown open
                    end try
                    return "ERROR: no AXMenuItem titled " & optionLabel & " found after opening the popup"
                end if
                -- `click itemEl` (System Events' generic verb) did not
                -- change the underlying value in the previous recon round
                -- (readback stayed "Dark"/"English" despite "SELECTED: ..."
                -- - the click itself succeeded structurally but nothing
                -- downstream reacted). Try the explicit AXPress action
                -- instead - `click` on a UI element is documented as
                -- roughly equivalent to `perform action "AXPress"`, but
                -- they are not guaranteed identical for every AXRole, and
                -- click_button's own success has only ever been observed
                -- via `click`, never compared against explicit AXPress for
                -- a menu-item-shaped role.
                try
                    perform action "AXPress" of itemEl
                on error errMsg
                    try
                        key code 53
                    end try
                    return "ERROR: AXPress AXMenuItem " & optionLabel & ": " & errMsg
                end try
                return "SELECTED: " & optionLabel

            else if cmdName is "type_into" then
                -- Text-field counterpart to select_popup_item: `set value
                -- of e to ...` was also confirmed a silent no-op for the
                -- font-size spinner. Focuses the field, selects all
                -- existing text (Cmd+A) and types the replacement - real
                -- keystroke synthesis, which M5-A's Linux harness found
                -- unreliable only because bare Xvfb has no window manager
                -- and X11 focus does not imply widget focus; this runner is
                -- a real GUI session (per the M5-A evidence doc) where
                -- System Events UI scripting has so far needed no special
                -- handling, so keystroke delivery to a genuinely focused
                -- field is a materially different situation.
                set roleWanted to item 3 of argv
                set n to (item 4 of argv) as integer
                set newText to item 5 of argv
                set idx to 0
                set allEl to my safeContents(w)
                set target to missing value
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is roleWanted then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            set target to e
                            exit repeat
                        end if
                    end if
                end repeat
                if target is missing value then return "NOT_FOUND"
                -- First attempt used `set focused of target to true` (an
                -- accessibility-attribute write) and the keystrokes landed
                -- nowhere. Second attempt added `click target` first (a
                -- real click event, same as click_button relies on) and
                -- still nothing changed - `keystroke` sends to whichever
                -- application is frontmost, which is not necessarily
                -- `procName` just because `tell process procName` scoped
                -- the click to it; `osascript`'s own process (or whatever
                -- last had focus) may still be frontmost. Explicitly
                -- activate the target process first - the one thing not
                -- yet tried and the standard fix for "keystroke goes
                -- nowhere" in AppleScript UI scripting.
                try
                    set frontmost of process procName to true
                end try
                delay 0.2
                -- Third attempt: frontmost + `click target` (the UI-
                -- element-reference form, which drives the accessibility
                -- bridge's AXPress-equivalent) still left keystrokes
                -- landing nowhere. `click target` is not necessarily the
                -- same thing as a genuine mouse event at that element's
                -- screen position - for a button, AXPress firing the same
                -- onclick handler a real click would is enough; a text
                -- field's actual OS-level key-window/first-responder focus
                -- may specifically require a real positional click.
                -- `click at {x, y}` (System Events' location-based form)
                -- synthesizes an actual mouse click, not an accessibility
                -- action - try that instead, at the field's own center.
                try
                    set p to position of target
                    set sz to size of target
                    set cx to (item 1 of p) + ((item 1 of sz) / 2)
                    set cy to (item 2 of p) + ((item 2 of sz) / 2)
                on error errMsg
                    return "ERROR: position/size: " & errMsg
                end try
                try
                    click at {cx, cy}
                on error errMsg
                    return "ERROR: click-at-coords: " & errMsg
                end try
                delay 0.2
                try
                    keystroke "a" using {command down}
                    delay 0.1
                    keystroke newText
                on error errMsg
                    return "ERROR: keystroke: " & errMsg
                end try
                delay 0.2
                try
                    return "TYPED: " & (value of target)
                on error errMsg
                    return "ERROR: readback: " & errMsg
                end try

            else if cmdName is "close_window" then
                try
                    click button 1 of w
                    return "CLOSED"
                on error errMsg
                    return "ERROR: " & errMsg
                end try

            else if cmdName is "scroll_area_bars" then
                -- M5-C / P03's horizontal-scroll-mirror check: a role
                -- tally against a wide-line fixture found `AXScrollArea=1`
                -- but `AXScrollBar=0` in the ordinary CHILD tree walk
                -- (`safeContents`/`UI elements of`) - standard
                -- NSAccessibility exposes a scroll area's bars as
                -- ATTRIBUTE-valued UI element references
                -- (`AXHorizontalScrollBar`/`AXVerticalScrollBar`), not as
                -- ordinary children reachable by walking `UI elements of`,
                -- which is exactly what a plain child-tree walk would miss.
                -- Finds the nth AXScrollArea (safeContents role filter,
                -- already proven reliable) and reads both scrollbar
                -- attributes directly off it, each with its own AXValue.
                set n to (item 3 of argv) as integer
                set allEl to my safeContents(w)
                set idx to 0
                set target to missing value
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is "AXScrollArea" then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            set target to e
                            exit repeat
                        end if
                    end if
                end repeat
                if target is missing value then return "NOT_FOUND"
                set hResult to "absent"
                try
                    set hBar to (value of attribute "AXHorizontalScrollBar" of target)
                    if hBar is missing value then
                        set hResult to "missing"
                    else
                        set hVal to "?"
                        try
                            set hVal to (value of hBar) as string
                        on error eh
                            set hVal to "ERR:" & eh
                        end try
                        set hResult to "value=" & hVal
                    end if
                on error eh2
                    set hResult to "ERROR:" & eh2
                end try
                set vResult to "absent"
                try
                    set vBar to (value of attribute "AXVerticalScrollBar" of target)
                    if vBar is missing value then
                        set vResult to "missing"
                    else
                        set vVal to "?"
                        try
                            set vVal to (value of vBar) as string
                        on error ev
                            set vVal to "ERR:" & ev
                        end try
                        set vResult to "value=" & vVal
                    end if
                on error ev2
                    set vResult to "ERROR:" & ev2
                end try
                return "SCROLLBARS: horizontal=" & hResult & " vertical=" & vResult

            else if cmdName is "set_scroll_bar" then
                -- Companion write to `scroll_area_bars`'s read: sets the
                -- nth AXScrollArea's named scrollbar (AXHorizontalScrollBar
                -- or AXVerticalScrollBar) AXValue directly - same direct-
                -- AXValue-write technique `set_value` already established
                -- for other controls, applied to the attribute-valued
                -- scrollbar reference `scroll_area_bars` found rather than
                -- an ordinary child element.
                set n to (item 3 of argv) as integer
                set whichBar to item 4 of argv
                set newVal to (item 5 of argv) as real
                set allEl to my safeContents(w)
                set idx to 0
                set target to missing value
                repeat with e in allEl
                    set isMatch to false
                    try
                        if role of e is "AXScrollArea" then set isMatch to true
                    end try
                    if isMatch then
                        set idx to idx + 1
                        if idx is n then
                            set target to e
                            exit repeat
                        end if
                    end if
                end repeat
                if target is missing value then return "NOT_FOUND"
                try
                    set bar to (value of attribute whichBar of target)
                    if bar is missing value then return "MISSING_BAR"
                    set value of bar to newVal
                    set afterVal to (value of bar) as string
                    return "SET: " & afterVal
                on error errMsg
                    return "ERROR: " & errMsg
                end try

            else if cmdName is "list_roles" then
                -- M5-C / F63 investigation: a cheap, bounded alternative to
                -- `dump_roles`'s abandoned bulk dump (see that command's own
                -- comment for why it wedges). Tallies occurrences of a FIXED
                -- small set of roles of interest - not an open-ended tally
                -- over every distinct role string, which would need
                -- per-element list membership checks against a growing list
                -- and get slow the same way `dump_roles` did. Answers the
                -- specific F63 question this exists for: does the diff
                -- view's scroll container expose a real AXScrollArea/
                -- AXScrollBar at all, distinct from just "how many AXRow
                -- exist right now" (which count_rows already answers).
                set allEl to my safeContents(w)
                set rowN to 0
                set scrollAreaN to 0
                set scrollBarN to 0
                set staticTextN to 0
                set groupN to 0
                set webAreaN to 0
                set totalN to 0
                repeat with e in allEl
                    set totalN to totalN + 1
                    set rl to ""
                    try
                        set rl to role of e
                    end try
                    if rl is "AXRow" then
                        set rowN to rowN + 1
                    else if rl is "AXScrollArea" then
                        set scrollAreaN to scrollAreaN + 1
                    else if rl is "AXScrollBar" then
                        set scrollBarN to scrollBarN + 1
                    else if rl is "AXStaticText" then
                        set staticTextN to staticTextN + 1
                    else if rl is "AXGroup" then
                        set groupN to groupN + 1
                    else if rl is "AXWebArea" then
                        set webAreaN to webAreaN + 1
                    end if
                end repeat
                return "AXRow=" & rowN & " AXScrollArea=" & scrollAreaN & " AXScrollBar=" & scrollBarN & " AXStaticText=" & staticTextN & " AXGroup=" & groupN & " AXWebArea=" & webAreaN & " total=" & totalN

            else if cmdName is "send_key" then
                -- M5-C / F63 investigation, and reused by P03's horizontal-
                -- scroll-mirroring check: a keyboard-driven scroll, distinct
                -- from `type_into`'s keystroke-of-text technique. Activates
                -- the target process first (the same fix `type_into` needed
                -- - `keystroke`/`key code` go to whichever app is actually
                -- frontmost, not merely whichever `tell process` scoped an
                -- earlier query to) then sends the given key code
                -- `repeatN` times with a short delay between presses, long
                -- enough for the WebView's own scroll/paint to keep up
                -- rather than coalescing every press into one jump.
                -- Key codes used by callers: 121 = Page Down, 125 = Down
                -- Arrow, 124 = Right Arrow (see Apple's Events.h list).
                set keyCode to (item 3 of argv) as integer
                set repeatN to (item 4 of argv) as integer
                try
                    set frontmost of process procName to true
                end try
                delay 0.2
                repeat repeatN times
                    try
                        key code keyCode
                    on error errMsg
                        return "ERROR: key code " & keyCode & ": " & errMsg
                    end try
                    delay 0.15
                end repeat
                return "DONE"

            else if cmdName is "dump_roles" then
                -- NOTE: deliberately five separate `try`-wrapped property
                -- fetches per element, matching count_rows/find_text/
                -- click_button's already-proven-reliable style, NOT a bulk
                -- `properties of e` record fetch - an earlier version tried
                -- that "fewer round trips" optimisation and it broke every
                -- single element on every view, including the CLI-compare
                -- view M5-A's commands already prove `entire contents`/
                -- per-property fetches work on ("AppleEvent handler failed
                -- (-10000)", persisting across all of `run`'s retries -
                -- consistent with `properties of e` wedging the whole
                -- System Events connection for the rest of that osascript
                -- process, not just failing for one element). Left as a
                -- documented dead end rather than silently dropped.
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
                    -- Deliberately no `enabled of e` here - querying it
                    -- unconditionally across every element (not just a
                    -- button already matched by role, which is all
                    -- click_button ever asks it of) reproduced the same
                    -- "-10000" failure this whole investigation chased,
                    -- even with plain `entire contents` on the same view
                    -- M5-A's commands already prove works. Left unexplained
                    -- (a WebKit/System Events accessibility-bridge quirk
                    -- for attributes some element kinds don't support, is
                    -- the best guess) but confirmed as the actual trigger.
                    set end of outLines to rl & "|" & ti & "|" & de & "|" & va
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
end runOnce

-- See the M5-B comment above `on run argv` for why this exists instead of
-- a plain `entire contents of el`. Both handlers wrap their own `tell
-- application "System Events"` rather than relying on the caller's -
-- "UI elements of"/"entire contents of"/etc. are System Events terminology
-- that only resolves at compile time inside a `tell` block for that
-- application; a top-level handler called via `my` from inside runOnce's
-- `tell` block does NOT inherit that terminology context (confirmed the
-- hard way: without this, osascript fails to *compile* the script at all,
-- "Expected "," but found property. (-2741)", on every command, not just
-- the ones that use these handlers).
on safeContents(el)
    tell application "System Events"
        try
            return (entire contents of el)
        on error
            set outList to {}
            my flatWalk(el, 30, outList)
            return outList
        end try
    end tell
end safeContents

on flatWalk(el, maxDepth, outList)
    if maxDepth < 0 then return
    tell application "System Events"
        try
            set kids to (UI elements of el)
        on error
            -- This node refuses to enumerate its children (the poison-node
            -- case `safeContents` exists for) - treat it as a leaf rather
            -- than failing the whole walk.
            return
        end try
    end tell
    repeat with k in kids
        set end of outList to k
        my flatWalk(k, maxDepth - 1, outList)
    end repeat
end flatWalk
