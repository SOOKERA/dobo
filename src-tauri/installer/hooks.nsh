; ${APP_NAME} Setup: run by Tauri's NSIS installer at fixed points (tauri.conf.json:
; bundle.windows.nsis.installerHooks). This file is included near the top of Tauri's script, before
; its own definitions: code outside the macros can't use ${PRODUCTNAME} and friends (so "${APP_NAME}" is
; written out), the macros can (they are expanded where Tauri inserts them).

; ${APP_NAME} Setup's own window (the slides, the options, the progress, the uninstaller's look).
; [APP] The app's own settings (name, exe, colours, switches...): copy app.example.nsh to app.nsh.
!include "${__FILEDIR__}\app.nsh"
; [APP] French (template 1.1.0): off unless app.nsh says !define APP_FRENCH 1. With it on, every text
; app.nsh gives needs its French twin (<name>_FR), and the build stops here naming any missing one.
!ifndef APP_FRENCH
  !define APP_FRENCH 0
!endif
; [APP] Optional (template 1.1.0): what the uninstaller says when the app's --uninstall-cleanup
; kept the data because another copy of the app is open (its exit code 2). An app.nsh that sets
; APP_UN_KEPT sets APP_UN_KEPT_FR too when French is on.
!ifndef APP_UN_KEPT
  !define APP_UN_KEPT "Another ${APP_NAME} is open, so ${APP_NAME}'s settings and lists were kept. To delete them, close that ${APP_NAME}, then delete the ${APP_NAME} folder in %LOCALAPPDATA%."
  !ifndef APP_UN_KEPT_FR
    !define APP_UN_KEPT_FR "Un autre ${APP_NAME} est ouvert : les paramètres et listes de ${APP_NAME} ont été gardés. Pour les supprimer, fermez cet autre ${APP_NAME}, puis supprimez le dossier ${APP_NAME} dans %LOCALAPPDATA%."
  !endif
!endif
; [APP] Optional (template 1.1.1): the dark look's accent, for an app whose colour is too dark to
; read on it (the art config's accentDark says the same). Default: APP_ACCENT in both looks.
!ifndef APP_ACCENT_DARK
  !define APP_ACCENT_DARK ${APP_ACCENT}
!endif
; [APP] Optional (template 1.1.1): the colour of the language not shown in English | Français:
; "accent" (like the window's links, the default) or "grey" (for an accent close to the text
; colour, where the two words would look the same).
!ifndef APP_LANG_OTHER
  !define APP_LANG_OTHER "accent"
!endif
!if "${APP_LANG_OTHER}" == "grey"
  !define APP_LANG_OTHER_COLOR MUT
!else if "${APP_LANG_OTHER}" == "accent"
  !define APP_LANG_OTHER_COLOR ACCENT
!else
  !error 'APP_LANG_OTHER is "${APP_LANG_OTHER}": write "accent" or "grey".'
!endif
!if ${APP_FRENCH} == 1
  !macro AppNeedFr NAME
    !ifndef ${NAME}_FR
      !error "APP_FRENCH is on: app.nsh needs ${NAME}_FR, the French twin of ${NAME}."
    !endif
  !macroend
  !insertmacro AppNeedFr APP_TAGLINE
  !insertmacro AppNeedFr APP_IS_OPEN_NOTE
  !insertmacro AppNeedFr APP_UN_KEEPS
  !insertmacro AppNeedFr APP_UN_DATA_SUB
  !insertmacro AppNeedFr APP_UN_DONE_TEXT
  !insertmacro AppNeedFr APP_CHOICES_FAILED
  !insertmacro AppNeedFr APP_UN_KEPT
  ; The extra-data switch's words: only when it is shown.
  !if ${APP_EXTRA_DATA_SWITCH} == 1
    !insertmacro AppNeedFr APP_EXTRA_DATA_LABEL
    !insertmacro AppNeedFr APP_EXTRA_DATA_SUB
  !else
    !define /ifndef APP_EXTRA_DATA_LABEL_FR ""
    !define /ifndef APP_EXTRA_DATA_SUB_FR ""
  !endif
!endif
!include "${__FILEDIR__}\ui.nsh"
; [APP] Both set up the window before it shows (installer/ui.nsh).
!define MUI_CUSTOMFUNCTION_GUIINIT AppGuiInit
!define MUI_CUSTOMFUNCTION_UNGUIINIT un.AppGuiInit

; ---------- where ${APP_NAME} is installed ----------
; Tauri's per-user default is %LOCALAPPDATA%\${APP_NAME}. [APP] ${APP_NAME} keeps its notes elsewhere
; (APP_DATA_DIR, app.nsh), but goes to %LOCALAPPDATA%\Programs\${APP_NAME} all the same (where
; Windows puts per-user programs), unless
; an earlier install is elsewhere (Tauri remembers it) or the folder was chosen with /D=.
; (Called by AppGuiInit, and for silent and passive installs.)

Function AppInstallDir
  Push $0
  ${If} $INSTDIR == "$LOCALAPPDATA\${APP_NAME}"
    ReadRegStr $0 HKCU "Software\${APP_PUBLISHER}\${APP_NAME}" ""
    ${If} $0 == ""
      StrCpy $INSTDIR "$LOCALAPPDATA\Programs\${APP_NAME}"
    ${EndIf}
  ${EndIf}
  Pop $0
FunctionEnd

; The window's folder (installer/ui.nsh: AppCheckFolder) also refuses a folder ${APP_NAME} keeps data
; in: [APP] %APPDATA%\${APP_IDENTIFIER} (the notes) and %LOCALAPPDATA%\${APP_IDENTIFIER} (WebView2's
; data), and a portable ${APP_NAME}'s folder (it has its own data folder: installing there would
; switch that copy to %APPDATA%\${APP_IDENTIFIER}, and its notes would look gone).

; ---------- a folder ${APP_NAME} can update itself in ----------
; ${APP_NAME} Setup installs without admin rights, and ${APP_NAME} updates itself by writing next to ${APP_EXE},
; without admin rights too: the folder must be one this Windows user can change. Windows only
; refuses a folder like Program Files once files are being copied ("Error opening file for
; writing"), so the window checks it as soon as a folder is shown or picked, and holds Install
; with the reason under the options (installer/ui.nsh: AppCheckFolder), and a silent or passive
; install checks it before anything is done (installer.nsi: .onInit).

; "" = fine; "write" = this user can't change the folder; "admin" = Setup runs as administrator
; and the folder is one only administrators can change (${APP_NAME}, run without, couldn't update there).
Var AppDirProblem
; With "write", why, when it's known (for the window's wording): "missing" = not even the drive
; or network folder is there, "long" = the path is too long for Windows; else "".
Var AppDirWhy

; Sets $AppDirProblem for $INSTDIR. Tried for real, with this Setup's rights (none asked for): a
; file of a unique name is made in $INSTDIR and deleted again or, when $INSTDIR isn't there yet,
; a folder with a file in it, in the nearest folder that is there (where Setup will make it).
Function AppCheckDir
  Push $0
  Push $1
  Push $2
  Push $3
  StrCpy $AppDirProblem ""
  StrCpy $AppDirWhy ""
  ; The folder in full ("..", "." and "/" resolved), without a "\" at the end (but "D:\").
  StrCpy $0 $INSTDIR
  ; A drive's own folder: NSIS keeps "D:\" as "D:", which Windows reads as D:'s current folder.
  StrLen $1 $0
  StrCpy $2 $0 1 1
  ${If} $1 = 2
  ${AndIf} $2 == ":"
    StrCpy $0 "$0\"
  ${EndIf}
  System::Call 'kernel32::GetFullPathNameW(w r0, i ${NSIS_MAX_STRLEN}, w .r3, p 0)i.r1'
  ; 0 = failed; NSIS_MAX_STRLEN or more = didn't fit (nothing was written).
  ${If} $1 = 0
  ${OrIf} $1 >= ${NSIS_MAX_STRLEN}
    StrCpy $3 $INSTDIR
  ${EndIf}
  StrLen $1 $3
  ${If} $1 > 3
    StrCpy $2 $3 1 -1
    ${IfThen} $2 == "\" ${|} StrCpy $3 $3 -1 ${|}
  ${EndIf}
  StrCpy $0 $3
  ; Asked by its attributes: a folder this user can't open is still there (FileExists says no).
  ${Do}
    System::Call 'kernel32::GetFileAttributesW(w r0)i.r1'
    ${If} $1 <> -1
      IntOp $1 $1 & 0x10 ; FILE_ATTRIBUTE_DIRECTORY
      ${IfThen} $1 = 0 ${|} StrCpy $AppDirProblem "write" ${|} ; a file
      ${Break}
    ${EndIf}
    ${GetParent} $0 $1
    ; Not even the drive or the network share is there. Above "\\server\share" GetParent gives
    ; "\\server", then "\": Windows reads "\" as the root of Setup's current drive, so the walk
    ; must stop there (the test folder was made, and left, in C:\ for an unreachable share).
    ${If} $1 == ""
    ${OrIf} $1 == $0
    ${OrIf} $1 == "\"
      StrCpy $AppDirProblem "write"
      StrCpy $AppDirWhy "missing"
      ${Break}
    ${EndIf}
    ; "D:" alone is D:'s current folder to Windows: its root is "D:\".
    StrLen $2 $1
    ${If} $2 = 2
      StrCpy $2 $1 1 1
      ${IfThen} $2 == ":" ${|} StrCpy $1 "$1\" ${|}
    ${EndIf}
    StrCpy $0 $1
  ${Loop}
  ${IfThen} $AppDirProblem != "" ${|} Goto app_check_done ${|}
  ${IfThen} $0 == "$3" ${|} StrCpy $0 "" ${|}
  ; "" = $INSTDIR is there; else the folder Setup will make it in.
  System::Call 'kernel32::GetCurrentProcessId()i.r1'
  System::Call 'kernel32::GetTickCount()i.r2'
  StrCpy $1 ".${APP_SLUG}-setup-check-$1-$2"
  ${If} $0 == ""
    StrCpy $2 $3 1 -1
    ${If} $2 == "\" ; a drive: "D:\"
      StrCpy $0 "$3$1.tmp"
    ${Else}
      StrCpy $0 "$3\$1.tmp"
    ${EndIf}
    ClearErrors
    FileOpen $2 $0 w
    ${If} ${Errors}
      StrCpy $AppDirProblem "write"
    ${Else}
      FileClose $2
      System::Call 'kernel32::DeleteFileW(w r0)i'
    ${EndIf}
  ${Else}
    ; One folder, made by Windows itself (NSIS' CreateDirectory would make every missing one).
    StrCpy $2 $0 1 -1
    ${If} $2 == "\" ; a drive: "D:\"
      StrCpy $0 "$0$1"
    ${Else}
      StrCpy $0 "$0\$1"
    ${EndIf}
    System::Call 'kernel32::CreateDirectoryW(w r0, p 0)i.r2'
    ${If} $2 = 0
      StrCpy $AppDirProblem "write"
    ${Else}
      ClearErrors
      FileOpen $2 "$0\check.tmp" w
      ${If} ${Errors}
        StrCpy $AppDirProblem "write"
      ${Else}
        FileClose $2
        StrCpy $2 "$0\check.tmp"
        System::Call 'kernel32::DeleteFileW(w r2)i'
      ${EndIf}
      ; Removed by Windows too: NSIS' RMDir refuses some paths CreateDirectoryW takes.
      System::Call 'kernel32::RemoveDirectoryW(w r0)i'
    ${EndIf}
  ${EndIf}
  ; Couldn't write there, with a path near Windows' limit (260, with the files ${APP_NAME} puts there).
  ${If} $AppDirProblem == "write"
    StrLen $1 $3
    ${IfThen} $1 > 200 ${|} StrCpy $AppDirWhy "long" ${|}
  ${EndIf}
  ; Run as administrator, Setup can write almost anywhere: what matters is whether ${APP_NAME} can,
  ; later, without. That can't be tried from here, so the folders Windows keeps for
  ; administrators are recognised by name.
  ${If} $AppDirProblem == ""
    System::Call 'shell32::IsUserAnAdmin()i.r1'
    ${If} $1 <> 0
      Push $3
      Call AppIsAdminFolder
      Pop $1
      ${IfThen} $1 = 1 ${|} StrCpy $AppDirProblem "admin" ${|}
    ${EndIf}
  ${EndIf}
  app_check_done:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

!macro AppIfUnder BASE
  StrLen $2 "${BASE}\"
  StrCpy $1 "$0\" $2
  ${IfThen} $1 == "${BASE}\" ${|} StrCpy $3 1 ${|}
!macroend

; Stack: a folder -> 1 if it is in a folder only administrators can change (a folder named
; "Program Files" or "Program Files (x86)" on any drive, or Windows' own Program Files or Windows
; folder), else 0. Windows ignores case in paths, and so does ==.
Function AppIsAdminFolder
  Exch $0
  Push $1
  Push $2
  Push $3
  StrCpy $3 0
  StrCpy $1 $0
  ${Do}
    ${GetFileName} $1 $2
    ${If} $2 == "Program Files"
    ${OrIf} $2 == "Program Files (x86)"
      StrCpy $3 1
      ${Break}
    ${EndIf}
    ${GetParent} $1 $2
    ${If} $2 == ""
    ${OrIf} $2 == $1
      ${Break}
    ${EndIf}
    StrCpy $1 $2
  ${Loop}
  !insertmacro AppIfUnder $WINDIR
  !insertmacro AppIfUnder $PROGRAMFILES
  !insertmacro AppIfUnder $PROGRAMFILES64
  StrCpy $0 $3
  Pop $3
  Pop $2
  Pop $1
  Exch $0
FunctionEnd

; A silent (/S) or passive (/P) install shows no folder page: the same check, at the end of
; .onInit, before anything is changed (an older version isn't removed either). A folder this user
; can't change ends Setup with exit code 2 and the message (in the console that started a silent
; Setup). One only administrators can change, with Setup run as administrator, is accepted
; there: no one can be asked.
!macro APP_CHECK_DIR_UNATTENDED
  ${If} ${Silent}
  ${OrIf} $PassiveMode = 1
    Call AppInstallDir
    Call AppCheckDir
    ${If} $AppDirProblem == "write"
      ${If} ${Silent}
        System::Call 'kernel32::AttachConsole(i -1)i.r0'
        ${If} $0 <> 0
          System::Call 'kernel32::GetStdHandle(i -11)i.r0'
          FileWrite $0 "$(appDirNotWritableSilent)"
        ${EndIf}
      ${Else}
        MessageBox MB_ICONSTOP|MB_OK "$(appDirNotWritable)"
      ${EndIf}
      SetErrorLevel 2
      Abort
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREINSTALL
  ; A silent install (/S) shows no window, so the function above hasn't run yet.
  ${If} ${Silent}
    Call AppInstallDir
    SetOutPath $INSTDIR
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; This copy is installed: ${APP_NAME} keeps its notes in %APPDATA%\${APP_IDENTIFIER}, never in the
  ; install folder (see src-tauri/src/paths.rs). Without this file ${APP_NAME} would act as the portable one.
  ClearErrors
  FileOpen $0 "$INSTDIR\${APP_MARKER}" w
  ${If} ${Errors}
    ; Said under the bar, with Close (installer/ui.nsh: AppFail).
    !insertmacro AppT $AppTx appFailInstall
    !insertmacro AppFail $AppTx
    Abort $AppTx
  ${EndIf}
  FileWrite $0 "Installed by ${APP_NAME} Setup. ${APP_NAME} keeps your notes in %APPDATA%\${APP_IDENTIFIER}.$\r$\n"
  FileClose $0
  ; The window's choices (never in a silent or passive install: ${APP_NAME} asks at its first start):
  ; ${APP_EXE} saves them in its settings and does them now (start with Windows, the browser
  ; connection), without a window (src-tauri/src/install.rs). Its first-start setup shows them.
  ; Waited for 15 s at most (it takes well under one); if it fails or doesn't end, "${APP_NAME} is
  ; installed" says so ($AppChoicesOk 0, installer/ui.nsh: AppDonePage).
  ${IfNot} ${Silent}
  ${AndIf} $AppPassiveOrig <> 1
    Push $1
    Push $2
    Push $3
    Push $4
    Push $5
    Push $6
    StrCpy $AppChoicesOk 0
    ; [APP] --terms-accepted=1: Install / Update was clicked under "By installing, you accept the
    ; terms of use", so ${APP_NAME} doesn't ask again at its first start (silent and passive installs
    ; never come here: nobody saw that line).
    !if ${APP_BROWSER_SWITCH} == 1
    StrCpy $1 '"$INSTDIR\${MAINBINARYNAME}.exe" --install-choices --start-with-windows=$AppStart --browsers=$AppBrowsers --terms-accepted=1'
    !else
    StrCpy $1 '"$INSTDIR\${MAINBINARYNAME}.exe" --install-choices --start-with-windows=$AppStart --terms-accepted=1'
    !endif
    ; [APP] --lang=en|fr: the window's language (apps with French on only), so the app starts in
    ; it (app-side/install.rs). Only when it was picked in the window. Nothing picked, the app
    ; keeps the language it has: its default for a new install (English, the window's first
    ; language too), or the one a portable copy already uses over the same data, an update, a
    ; reinstall… "Nothing saved yet" doesn't mean "chosen": a silent install saves nothing, and
    ; a portable copy's data may say French. (The installed app then notes its own language for
    ; the uninstaller: README, The app's language.)
    !if ${APP_FRENCH} == 1
    ${If} $AppLangPicked = 1
      StrCpy $1 '$1 --lang=$AppLang'
    ${EndIf}
    !endif
    ; STARTUPINFOW (68 bytes in a 32-bit Setup, 104 in a 64-bit one), PROCESS_INFORMATION.
    System::Alloc 104
    Pop $2
    !if ${NSIS_PTR_SIZE} > 4
      System::Call "*$2(i 104)"
    !else
      System::Call "*$2(i 68)"
    !endif
    System::Alloc 24
    Pop $3
    System::Call 'kernel32::CreateProcessW(p 0, w r1, p 0, p 0, i 0, i 0x08000000, p 0, p 0, p r2, p r3) i .r4'
    ${If} $4 <> 0
      System::Call "*$3(p .r5, p .r6)"
      System::Call 'kernel32::WaitForSingleObject(p r5, i 15000) i .r4'
      ${If} $4 = 0
        System::Call 'kernel32::GetExitCodeProcess(p r5, *i .r4)'
        ${IfThen} $4 = 0 ${|} StrCpy $AppChoicesOk 1 ${|}
      ${EndIf}
      System::Call 'kernel32::CloseHandle(p r5)'
      System::Call 'kernel32::CloseHandle(p r6)'
    ${EndIf}
    System::Free $2
    System::Free $3
    Pop $6
    Pop $5
    Pop $4
    Pop $3
    Pop $2
    Pop $1
  ${EndIf}
  ; Left by an update ${APP_NAME} made itself: this install replaces them (the new ${APP_EXE} must not be
  ; put "on trial" against an older copy, or told an update was put back).
  Delete "$INSTDIR\${MAINBINARYNAME}.old.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.bad.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.trial.json"
  Delete "$INSTDIR\${MAINBINARYNAME}.rolled-back.txt"
!macroend

; ---------- uninstalling ----------
; Most of the clean-up is done by ${APP_EXE} --uninstall-cleanup (src-tauri/src/install.rs): a ${APP_NAME}
; that updated itself keeps this uninstaller from the version first installed, but its ${APP_EXE} is
; always the current one, so what changes between versions belongs there.

; A "start with Windows" entry left by another ${APP_NAME} (a portable copy): Tauri deletes the "${APP_NAME}"
; value after PREUNINSTALL whatever it starts, so it's put back in POSTUNINSTALL.
Var AppOtherRun
; An app.nsh made before template 1.0.4 has no APP_START_ARG.
!ifndef APP_START_ARG
  !define APP_START_ARG "--tray"
!endif
; 1 = ${APP_EXE} couldn't delete the data: this uninstaller deletes what it knows itself.
Var AppDeleteHere
; The uninstall-cleanup switches ("--delete-data", "--delete-private").
Var AppUnArgs

!macro NSIS_HOOK_PREUNINSTALL
  ; ${APP_NAME} must be closed first (asks, unless the window said so already; a silent uninstall
  ; closes it).
  !insertmacro AppCheckRunning "$INSTDIR\${MAINBINARYNAME}.exe"
  StrCpy $AppOtherRun ""
  StrCpy $AppDeleteHere 0
  ; Never when Setup replaces this version (/UPDATE): only the files go then.
  ${If} $UpdateMode <> 1
    ; Removes the browser connection, ${APP_SLUG}:// links and start with Windows where they point at
    ; this copy. With "Also delete my notes" ticked (never in a silent or passive
    ; uninstall: no page, so no tick), it deletes them too, unless another ${APP_NAME} is open.
    ; "Also delete Private mode's files" is its own switch: never from the first one.
    ClearErrors
    StrCpy $AppUnArgs ""
    ${If} $DeleteAppDataCheckboxState = 1
      StrCpy $AppUnArgs " --delete-data"
    ${EndIf}
    ${If} $AppDeletePrivate = 1
      StrCpy $AppUnArgs "$AppUnArgs ${APP_EXTRA_DATA_FLAG}"
    ${EndIf}
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --uninstall-cleanup$AppUnArgs' $0
    ${If} ${Errors}
      StrCpy $0 1
    ${EndIf}
    ${If} $DeleteAppDataCheckboxState = 1
    ${OrIf} $AppDeletePrivate = 1
      ${If} $0 = 2
        ; Another ${APP_NAME} is open (a portable copy): it may use the same folders. Nothing is
        ; deleted, also not by Tauri below. [APP] Also when a portable Dobo uses these notes (it
        ; noted so in them: .dobo-portable-copies, src-tauri/src/paths.rs).
        StrCpy $DeleteAppDataCheckboxState 0
        StrCpy $AppDeletePrivate 0
        ; [APP] The words: English.nsh / French.nsh (appOtherOpen).
        !insertmacro AppT $AppTx appOtherOpen
        MessageBox MB_ICONINFORMATION|MB_OK $AppTx /SD IDOK
      ${ElseIf} $0 <> 0
        StrCpy $AppDeleteHere 1
      ${EndIf}
    ${EndIf}
    ; This copy's entry is gone by now: one still there is another ${APP_NAME}'s (unless ${APP_EXE}
    ; couldn't run).
    ReadRegStr $AppOtherRun HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}"
    ; Quoted or not, with or without APP_START_ARG (tauri-plugin-autostart writes it unquoted).
    ; [APP] Dobo 1.1 writes it quoted ("<exe>" --hidden); 1.0 wrote it unquoted.
    ${If} $AppOtherRun == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\""
    ${OrIf} $AppOtherRun == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" ${APP_START_ARG}"
    ${OrIf} $AppOtherRun == "$INSTDIR\${MAINBINARYNAME}.exe"
    ${OrIf} $AppOtherRun == "$INSTDIR\${MAINBINARYNAME}.exe ${APP_START_ARG}"
      StrCpy $AppOtherRun ""
    ${EndIf}
  ${EndIf}
!macroend

; [APP] Deletes everything in the folder on the stack except the extra data folders
; (APP_EXTRA_DATA_DIR, APP_EXTRA_TEMP_DIR; Pako: "private", "private-open") when the app has the
; extra-data switch: "Also delete ${APP_NAME}'s settings and lists" alone never deletes them.
Function un.AppDeleteButPrivate
  Exch $R0
  Push $R1
  Push $R2
  FindFirst $R1 $R2 "$R0\*"
  ${DoWhile} $R2 != ""
    ${If} $R2 != "."
    ${AndIf} $R2 != ".."
    !if ${APP_EXTRA_DATA_SWITCH} == 1
    ${AndIf} $R2 != "${APP_EXTRA_DATA_DIR}"
    ${AndIf} $R2 != "${APP_EXTRA_TEMP_DIR}"
    !endif
      ${If} ${FileExists} "$R0\$R2\*"
        RMDir /r "$R0\$R2"
      ${Else}
        Delete "$R0\$R2"
      ${EndIf}
    ${EndIf}
    FindNext $R1 $R2
  ${Loop}
  FindClose $R1
  RMDir "$R0"
  Pop $R2
  Pop $R1
  Pop $R0
FunctionEnd

!macro NSIS_HOOK_POSTUNINSTALL
  ; Files ${APP_NAME} writes next to itself (its own updates), so the folder can be removed. Only now:
  ; until Tauri's last check that ${APP_NAME} is closed, this copy must still be the installed one.
  Delete "$INSTDIR\${APP_MARKER}"
  Delete "$INSTDIR\${MAINBINARYNAME}.new.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.new.*.part"
  Delete "$INSTDIR\${MAINBINARYNAME}.old.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.bad.exe"
  Delete "$INSTDIR\${MAINBINARYNAME}.update.json"
  Delete "$INSTDIR\${MAINBINARYNAME}.trial.json"
  Delete "$INSTDIR\${MAINBINARYNAME}.rolled-back.txt"
  ; [APP] The update's "can this folder be changed?" test file (updates.rs folder_writable), left
  ; only if Dobo stopped right between making and deleting it.
  Delete "$INSTDIR\.${APP_SLUG}-write-test-*"
  RMDir "$INSTDIR"
  ${If} $UpdateMode <> 1
    ${If} $AppOtherRun != ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_NAME}" $AppOtherRun
    ${EndIf}
    ; ${APP_EXE} couldn't delete the data: [APP] Dobo's two folders (app.nsh: APP_DATA_DIR, the
    ; notes; APP_WEBVIEW_DIR, WebView2's), only with "Also delete my notes" ticked, never in a
    ; silent uninstall (no page, so no tick). Tauri's own "delete app data" (installer.nsi) removes
    ; the same two folders, named by the identifier.
    ${If} $AppDeleteHere = 1
    ${AndIf} $DeleteAppDataCheckboxState = 1
    ${AndIfNot} ${Silent}
      RMDir /r "${APP_DATA_DIR}"
      RMDir /r "${APP_WEBVIEW_DIR}"
    ${EndIf}
  ${EndIf}
!macroend
