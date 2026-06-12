# Troubleshooting

## Diagnosing startup problems

Run `forskscope --diagnostics` from a terminal before filing a bug report.
This prints platform information (OS, architecture, CPU count, app version,
Rust version) and exits without launching the UI. Copy the output into your
bug report.

```sh
forskscope --diagnostics
```

Example output:

```
ForskScope 0.112.0
OS:       linux
Arch:     x86_64
CPUs:     8
Rust:     1.85.0
Home:     ***
Config:   /home/user/.config/forskscope
```

---

## Linux: app fails to start — missing WebView library

**Symptom:** The app exits immediately with an error like:
```
error while loading shared libraries: libwebkit2gtk-4.1.so.0: cannot open
shared object file: No such file or directory
```

**Cause:** ForskScope uses Dioxus Desktop, which requires **WebKitGTK 4.1**
at runtime. Some distributions package only WebKitGTK 4.0 (`libwebkit2gtk-4.0`)
by default.

**Fix — Debian / Ubuntu:**
```sh
sudo apt-get install libwebkit2gtk-4.1-0 libgtk-3-0
```

If `libwebkit2gtk-4.1-0` is not found:
```sh
apt-cache search webkit2gtk
```
Look for a `4.1` variant. On Ubuntu 20.04 / Debian Bullseye you may need to
add a newer package source or use a distribution that ships WebKitGTK 4.1
(Ubuntu 22.04+, Debian Bookworm+).

**Fix — Fedora / RHEL / Rocky:**
```sh
sudo dnf install webkit2gtk4.1 gtk3
```

**Fix — Arch Linux:**
```sh
sudo pacman -S webkit2gtk-4.1 gtk3
```

---

## Linux: blank white window on launch

**Symptom:** The app opens but shows a blank white window; no content
appears.

**Possible causes and fixes:**

1. **NVIDIA GPU + Wayland / DMA-BUF issue.** Try disabling the DMA-BUF
   renderer:
   ```sh
   WEBKIT_DISABLE_DMABUF_RENDERER=1 forskscope
   ```
   If this fixes it, add the variable to your shell profile or create a
   wrapper script.

2. **Wrong WebKitGTK version.** Check which version is installed:
   ```sh
   pkg-config --modversion webkit2gtk-4.1
   ```
   If the command fails, WebKitGTK 4.1 is not installed. See the section
   above.

3. **Wayland compositor issue.** Try running under X11:
   ```sh
   GDK_BACKEND=x11 forskscope
   ```

---

## Linux: file picker dialog does not open

**Symptom:** Clicking "Open files" or using drag-and-drop does nothing, or
the app hangs briefly and nothing appears.

**Cause:** The file picker (`rfd`) uses the OS portal on Wayland and the
GTK file chooser on X11. Missing portal or XDG desktop portal services can
block it.

**Fix:**
```sh
# Install the portal service:
sudo apt-get install xdg-desktop-portal xdg-desktop-portal-gtk  # Debian/Ubuntu
sudo dnf install xdg-desktop-portal xdg-desktop-portal-gtk      # Fedora
```

Then restart your desktop session.

---

## macOS: "ForskScope cannot be opened because the developer cannot be verified"

**Symptom:** macOS Gatekeeper blocks the first launch.

**Fix:** Right-click the app icon → **Open**, then click **Open** in the
dialog. Alternatively, from a terminal:
```sh
xattr -d com.apple.quarantine ./forskscope
```

This only needs to be done once.

---

## macOS: app crashes after macOS upgrade

If ForskScope was working before an OS upgrade and stops working after,
the WebView runtime may need to be reset. Try relaunching; if that does
not help, reinstall from the latest release.

---

## Windows: app does not start — WebView2 missing

**Symptom:** An error dialog mentions `WebView2` or `msedgewebview2.exe`.

**Cause:** Dioxus Desktop uses the Microsoft Edge WebView2 runtime on
Windows.

**Fix:** Download and install the WebView2 Evergreen Runtime from
[Microsoft](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
The "Evergreen Standalone Installer" is the easiest option.

---

## Windows: long path issues

If you see errors opening files in deep directory structures, enable long
path support:

1. Open **Group Policy Editor** (`gpedit.msc`).
2. Navigate to **Computer Configuration → Administrative Templates →
   System → Filesystem**.
3. Enable **Enable Win32 long paths**.

Or via PowerShell (as Administrator):
```powershell
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Control\FileSystem" `
    -Name "LongPathsEnabled" -Value 1
```

---

## Session not restored on restart

ForskScope saves the open tabs when the tab list changes. If the session
is not restored:

1. Check that the config directory is writable:
   - Linux: `~/.config/forskscope/`
   - macOS: `~/Library/Application Support/forskscope/`
   - Windows: `%APPDATA%\forskscope\`

2. Run `forskscope --diagnostics` to confirm the config path the app is
   using.

---

## Filing a bug report

Include:

- Output of `forskscope --diagnostics`
- Operating system and version
- Desktop environment (GNOME, KDE, etc.) and display server (Wayland / X11)
- How to reproduce the problem
- What you expected vs what happened

See [CONTRIBUTING.md](../../../CONTRIBUTING.md) for the issue tracker link.
