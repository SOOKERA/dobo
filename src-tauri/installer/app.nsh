; Dobo's settings for the Sookera installer template (template 1.1.3, from its examples/ztodo):
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
!define APP_TERMS_URL_FR "https://sookera.com/fr/terms"   ; the terms link in French (template 1.1.3)
!define APP_TAGLINE "Tasks and ideas on a calm board of notes"
; French (template 1.1.0): Setup's main page offers English | Français (English first), and every
; text below has its French twin (<name>_FR), in Dobo's own French (src/lib/i18n/fr.ts glossary:
; appli, never « app »). The French treats the name as masculine (« Dobo est installé »).
!define APP_FRENCH 1
!define APP_TAGLINE_FR "Tâches et idées, au calme, sur un tableau de notes"

; Dobo's ink on paper (the logo's #1f1d1a). On Setup's dark look the ink would vanish, so there
; the accent is Dobo's paper (#fffcf4, the note), as the app's own dark mode swaps them
; (APP_ACCENT_DARK, template 1.1.1; art/dobo.config.mjs says the same).
!define APP_ACCENT 0x1f1d1a
!define APP_ACCENT_DARK 0xfffcf4
; The accent is the text's own colour on both looks (ink, paper), so in English | Français the
; language not shown is grey: in the accent it would look the same as the one shown (template 1.1.1).
!define APP_LANG_OTHER "grey"
!define APP_SLIDES 5                               ; art/dobo.config.mjs makes 5

!define APP_DEFAULT_DESKTOP 1
!define APP_DEFAULT_START 1
!define APP_START_ARG "--hidden"                   ; what Dobo adds after its exe in that entry (install.rs)
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
!define APP_UN_DATA_SUB "Your notes, pictures, settings and the backups in Dobo’s own folder. A backups folder you chose is kept. This can’t be undone."
!define APP_UN_DONE_TEXT "Dobo is gone from this PC."
!define APP_CHOICES_FAILED "“Start Dobo with Windows” couldn’t be set. Turn it on in Dobo’s Settings."
; [APP] The uninstaller's message when Dobo.exe --uninstall-cleanup kept the notes (exit code 2:
; another Dobo is open, or a portable Dobo uses these notes, paths.rs PORTABLE_MARKER).
!define APP_UN_KEPT "Your notes were kept: another ${APP_NAME} is open, or a portable ${APP_NAME} uses them too. To delete them anyway, close every ${APP_NAME}, then delete the ${APP_IDENTIFIER} folder in %APPDATA%."

!define APP_IS_OPEN_NOTE_FR "Vos notes sont enregistrées pendant que vous écrivez : rien n’est perdu."
!define APP_UN_KEEPS_FR "Rien d’autre n’est touché sur votre PC."
!define APP_UN_DATA_SUB_FR "Vos notes, images, paramètres et les sauvegardes du dossier de Dobo. Un dossier de sauvegardes que vous avez choisi est gardé. C’est définitif."
!define APP_UN_DONE_TEXT_FR "Dobo n’est plus sur ce PC."
!define APP_CHOICES_FAILED_FR "« Démarrer Dobo avec Windows » n’a pas pu être réglé. Activez-le dans les Paramètres de Dobo."
!define APP_UN_KEPT_FR "Vos notes ont été gardées : un autre ${APP_NAME} est ouvert, ou un ${APP_NAME} portable les utilise aussi. Pour les supprimer quand même, fermez tous les ${APP_NAME}, puis supprimez le dossier ${APP_IDENTIFIER} dans %APPDATA%."
