# Privacy Policy

**Effective:** 2026-10-14.

This policy covers ForskScope on every platform, however you installed it: from
GitHub releases, the AUR, or the Microsoft Store, or built from source.

**In short:**

- ForskScope collects no personal information, and your files never leave your
  device.
- It makes one kind of network request, and only when you click *Check for
  updates*.

## 1. Your files stay on your device

ForskScope compares and merges files and folders on your computer. It reads and
writes only the files and folders you open, and it never uploads them or any part
of them.

## 2. What ForskScope keeps, and where

ForskScope stores the following on your device, in your user configuration and
data folders:

- **Settings:** theme, language, compare profiles, and the folders last shown in
  the Explorer.
- **Session:** the paths of the file comparisons that were open, so they can
  reopen next time.
- **Batch-copy manifests:** a record of each batch copy you run.

When you save a merge, ForskScope may also keep a backup copy of the previous
file next to it.

None of this is sent anywhere. You can delete these files at any time.

## 3. Network access: only when you click *Check for updates*

ForskScope makes no network requests on its own:

- none at startup and none in the background;
- no telemetry, no analytics and no crash reports.

**The one exception is the *Check for updates* button in the About dialog.** When
you click it, ForskScope:

- sends one HTTPS request to GitHub (`api.github.com`) to read the version number
  of the latest release;
- identifies itself in that request as `ForskScope/<version>`;
- sends nothing about your files or your settings;
- does not store the result.

Like any internet request, it reveals your IP address to GitHub. GitHub handles
the request under its
[privacy statement](https://docs.github.com/site-policy/privacy-policies/github-general-privacy-statement).

If you then click *Open release page*, your web browser opens GitHub's release
page. That visit is between your browser and GitHub.

## 4. Diagnostics you choose to share

*Copy diagnostics* in the About dialog copies a short report to your clipboard:

- the app version and the Rust version;
- the operating system, the architecture, and the number of CPUs;
- the paths of your home and configuration folders.

The report is not sent anywhere. If you paste it into a bug report, read it
first: the folder paths can include your user name.

## 5. System access

**On Windows,** ForskScope declares the `runFullTrust` capability, as desktop
applications do, so that it can open the files and folders you choose. All file
and configuration work happens on your device.

ForskScope draws its window with your system's web rendering component:

| System | Component |
|---|---|
| Windows | WebView2 |
| Linux | WebKitGTK |
| macOS | WKWebView |

It shows only content built into the app, never a web page from the internet.

## 6. The website

ForskScope's website, <https://forskscope.github.io/forskscope/>, is hosted on
GitHub Pages.

- It has no analytics, sets no cookies, and loads nothing from other sites.
- The user guide remembers your theme and sidebar choices in your browser's local
  storage.
- GitHub, as the host, processes your requests to the site under its privacy
  statement.

## 7. Changes to this policy

A changed policy is published here, with a new effective date, and the change is
recorded in the project's history.

## 8. Contact

For questions, open an issue at <https://github.com/forskscope/forskscope/issues>.
Issues are public, so please do not include personal information in them.
