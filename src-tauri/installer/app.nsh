; Dobo's settings for the Sookera installer template (template 1.0.3, from its examples/ztodo):
; Desktop shortcut on, Start with Windows ON by default (Dobo's own first start has it on too, so
; an installed and a portable copy start the same way), no browser switch, no extra-data switch.
;
; Dobo's data is NOT in %LOCALAPPDATA%\<App> as the template assumes: the app was called ZToDo
; while it was made and its identifier (com.sookera.ztodo) names its folders, kept so the notes
; people already have stay put (src-tauri/src/paths.rs):
;   the notes:        %APPDATA%\com.sookera.ztodo       (APP_DATA_DIR below)
;   WebView2's data:  %LOCALAPPDATA%\com.sookera.ztodo  (APP_WEBVIEW_DIR below)
; hooks.nsh and ui.nsh use these two wherever the template had %LOCALAPPDATA%\<App>.
!define APP_NAME "Dobo"
!define APP_SLUG "dobo"
!define APP_EXE "Dobo.exe"
!define APP_IDENTIFIER "com.sookera.ztodo"         ; tauri.conf.json "identifier" (kept from ZToDo)
!define APP_PUBLISHER "Sookera"
!define APP_MARKER ".dobo-installed"               ; = paths.rs INSTALLED_MARKER
!define APP_TERMS_URL "https://sookera.com/terms"
!define APP_TAGLINE "Tasks and ideas on a calm board of notes"

; Dobo's ink on paper (the logo's #1f1d1a). On Setup's dark look the ink would vanish, so there
; the accent is Dobo's paper (#fffcf4, the note), as the app's own dark mode swaps them
; (ui.nsh: APP_ACCENT_DARK, an [APP] addition; art/dobo.config.mjs says the same).
!define APP_ACCENT 0x1f1d1a
!define APP_ACCENT_DARK 0xfffcf4
!define APP_SLIDES 5                               ; art/dobo.config.mjs makes 5

!define APP_DEFAULT_DESKTOP 1
!define APP_DEFAULT_START 1
!define APP_BROWSER_SWITCH 0
!define APP_DEFAULT_BROWSERS 0
!define APP_NATIVE_HOST ""

!define APP_EXTRA_DATA_SWITCH 0
!define APP_EXTRA_DATA_DIR ""
!define APP_EXTRA_TEMP_DIR ""
!define APP_EXTRA_DATA_FLAG ""
!define APP_EXTRA_DATA_LABEL ""
!define APP_EXTRA_DATA_SUB ""

; [APP] Dobo's own data folders (see the top of this file).
!define APP_DATA_DIR "$APPDATA\com.sookera.ztodo"
!define APP_WEBVIEW_DIR "$LOCALAPPDATA\com.sookera.ztodo"

!define APP_IS_OPEN_NOTE "Your notes are saved as you write: nothing is lost."
!define APP_UN_KEEPS "Nothing else on your PC is touched."
; (Exact: a backups folder chosen in Settings is the person's own and stays, install.rs.)
!define APP_UN_DATA_SUB "Your notes, pictures, settings and the backups in Dobo's own folder. A backups folder you chose is kept. This can't be undone."
!define APP_UN_DONE_TEXT "Dobo is gone from this PC."
!define APP_CHOICES_FAILED "Start with Windows couldn't be set. Turn it on in Dobo's Settings."
