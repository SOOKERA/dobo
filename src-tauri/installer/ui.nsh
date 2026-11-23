; ${APP_NAME} Setup's own window (user-approved design, 2026-11-18): one wide window that follows
; Windows' light or dark look. Left: five feature slides that slide on by themselves every 4.5 s
; (they wait while the mouse is on them; the dots and arrows flip them, and a click stops them).
; Right: ${APP_NAME}'s logo and every option at once (folder, desktop shortcut, start with Windows, the
; browser connection), one Install button and the terms line. Then the same window shows the
; progress, then "Open ${APP_NAME}". Over an installed ${APP_NAME} it is "Update ${APP_NAME}" (folder locked). The
; uninstaller has the matching look, smaller.
;
; Included by hooks.nsh; the functions are put in by installer.nsi ("!insertmacro
; APP_UI_FUNCTIONS", where Tauri's defines exist). The pictures are made by
; scripts/installer-art.mjs (art/), drawn at twice the size and scaled here to the screen.
; Silent (/S) and passive (/P) installs never show any of this.

; Template 1.1.1's optional colours, also here (hooks.nsh sets them first; a hooks.nsh kept from
; 1.1.0 doesn't): the dark look's accent and the language not shown.
!ifndef APP_ACCENT_DARK
  !define APP_ACCENT_DARK ${APP_ACCENT}
!endif
; The terms in French (template 1.1.3): the same page unless the app has a French one.
!ifndef APP_TERMS_URL_FR
  !define APP_TERMS_URL_FR "${APP_TERMS_URL}"
!endif
!ifndef APP_LANG_OTHER_COLOR
  !define APP_LANG_OTHER_COLOR ACCENT
!endif

!define APP_ART "${__FILEDIR__}\art"

!ifndef SS_NOTIFY
  !define SS_NOTIFY 0x100
!endif
!define APP_SS_CENTER 0x1
!define APP_SS_RIGHT 0x2
!define APP_SS_BITMAP 0xE
!define APP_SS_NOPREFIX 0x80
!define APP_SS_PATHELLIPSIS 0x8000
!define APP_SS_ENDELLIPSIS 0x4000
!define APP_LABEL 0x50000080 ; WS_CHILD | WS_VISIBLE | SS_NOPREFIX
!define APP_PIC 0x5000010E ; WS_CHILD | WS_VISIBLE | SS_BITMAP | SS_NOTIFY
; A picture the mouse goes through (no SS_NOTIFY), hidden at first: the progress page's Close,
; over Windows' own Cancel button.
!define APP_PIC_PASS 0x4000000E
; The keyboard's and screen readers' stand-ins (AppProxy): WS_CHILD | WS_VISIBLE |
; WS_CLIPSIBLINGS | WS_TABSTOP, a push button or a check box.
!define APP_PUSH 0x54010000
!define APP_CHECK 0x54010002
!define APP_HIDDEN_LABEL 0x40000080

; The slides' pictures are read before anything else (solid compression).
ReserveFile "${APP_ART}\*.jpg"
ReserveFile "${APP_ART}\*.png"

; [APP] French (app.nsh: APP_FRENCH 1) needs the art script's French pictures too (a slide's `fr`
; in the art config makes slideN-d-fr.jpg, the buttons' b-<name>-<state>-d-fr.png...).
!if ${APP_FRENCH} == 1
  !if ! /FileExists "${APP_ART}\slide1-d-fr.jpg"
    !error "APP_FRENCH is on but art\slide1-d-fr.jpg is missing: give every slide its fr: { title, desc } in the art config and run art/installer-art.mjs again."
  !endif
  !if ! /FileExists "${APP_ART}\b-install-n-d-fr.png"
    !error "APP_FRENCH is on but art\b-install-n-d-fr.png is missing: run art/installer-art.mjs again (with fr in the slides)."
  !endif
!endif

; ---------- the words, in English or French ----------
; [APP] English.nsh and French.nsh give each text twice: as Tauri's LangString (what $(name) reads,
; always in the language Setup started in: NSIS fixes it once .onInit is over) and as a define,
; AppEn_<name> / AppFr_<name>. Setup's own code reads the define for the language shown ($AppLang)
; through AppT, so the window can switch languages while it is open. The uninstaller and silent or
; passive runs know their language from the start, so both ways give the same words there.
Var AppLang          ; en or fr: the language Setup's window speaks
Var AppTx            ; AppT's text, for the line that uses it next
!if ${APP_FRENCH} == 1
Var AppTxExtract     ; NSIS's own "Copying " (^Extract) in the language shown (English.nsh)
!endif
Var AppArtSfx        ; "" or "-fr": the pictures with words in them, in the language shown
!define APP_LANG_EN_NAME "English"
!define APP_LANG_FR_NAME "Français"

!macro AppEn KEY TEXT
  !define AppEn_${KEY} "${TEXT}"
  LangString ${KEY} ${LANG_ENGLISH} "${TEXT}"
!macroend
!macro AppFr KEY TEXT
  !define AppFr_${KEY} "${TEXT}"
  LangString ${KEY} ${LANG_FRENCH} "${TEXT}"
!macroend

; OUT = the text KEY in the language shown. Only after English.nsh and French.nsh (installer.nsi
; includes them after the pages): code before that calls a function that uses it (AppRunTexts).
!macro AppT OUT KEY
  !ifndef AppEn_${KEY}
    !error "No English text ${KEY} (English.nsh), or AppT is used before the language files are included."
  !endif
  !if ${APP_FRENCH} == 1
    !ifndef AppFr_${KEY}
      !error "No French text ${KEY} in French.nsh."
    !endif
    ${If} $AppLang == "fr"
      StrCpy ${OUT} "${AppFr_${KEY}}"
    ${Else}
      StrCpy ${OUT} "${AppEn_${KEY}}"
    ${EndIf}
  !else
    StrCpy ${OUT} "${AppEn_${KEY}}"
  !endif
!macroend

; .onInit (installer.nsi): Setup speaks English until the window's choice says otherwise; it never
; follows Windows' language. Over an installed ${APP_NAME} the window opens in the language saved
; for it (AppGuiInit). A silent or passive install (no window, no choice) keeps the language
; saved before, so that the uninstaller keeps it too.
!macro APP_LANG_INIT
  StrCpy $AppLang "en"
  StrCpy $AppArtSfx ""
  !if ${APP_FRENCH} == 1
    !if "${DISPLAYLANGUAGESELECTOR}" == "true"
      !error "With APP_FRENCH, Setup's window asks the language itself: set displayLanguageSelector to false in tauri.conf.json."
    !endif
    StrCpy $LANGUAGE ${LANG_ENGLISH}
    ${If} ${Silent}
    ${OrIf} $PassiveMode = 1
      ReadRegStr $AppTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
      ${If} $AppTx == ${LANG_FRENCH}
        StrCpy $AppLang "fr"
        StrCpy $LANGUAGE ${LANG_FRENCH}
        StrCpy $AppArtSfx "-fr"
      ${EndIf}
    ${EndIf}
    Call AppLangTexts
  !endif
!macroend

; un.onInit (installer.nsi): the uninstaller speaks the language Setup ran in (saved by Setup as
; the app's "Installer Language", 1033 or 1036). Missing (installed before template 1.1.0) or
; anything else: English. Never Windows' language dialog.
!macro APP_UN_LANG_INIT
  StrCpy $AppLang "en"
  StrCpy $AppArtSfx ""
  !if ${APP_FRENCH} == 1
    StrCpy $LANGUAGE ${LANG_ENGLISH}
    ReadRegStr $AppTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
    ${If} $AppTx == ${LANG_FRENCH}
      StrCpy $AppLang "fr"
      StrCpy $LANGUAGE ${LANG_FRENCH}
      StrCpy $AppArtSfx "-fr"
    ${EndIf}
  !else
    !insertmacro MUI_UNGETLANGUAGE
  !endif
!macroend

Var AppDpi
Var AppLook          ; d(ark) or l(ight)
Var AppCBg
Var AppCText
Var AppCRa
Var AppFBody         ; 13 px
Var AppFSmall        ; 12 px
Var AppFTiny         ; 11 px
Var AppFTitle        ; 17 px semibold
Var AppX
Var AppY
Var AppW
Var AppH
Var AppDialog
Var AppGdip
; slides
Var AppS1
Var AppS2
Var AppS3
Var AppS4
Var AppS5
; [APP] AppS1..5 are the slides of the language shown: the English ones (AppSE) are read before
; the window shows, the French ones (AppSF) one by one while the window is idle (AppTick), or at
; once when French is picked before that.
Var AppSE1
Var AppSE2
Var AppSE3
Var AppSE4
Var AppSE5
!if ${APP_FRENCH} == 1
Var AppSF1
Var AppSF2
Var AppSF3
Var AppSF4
Var AppSF5
Var AppSFReady       ; how many French slides are read
!endif
Var AppSlide         ; 1..${APP_SLIDES}
Var AppAuto          ; 1 = moves on by itself
Var AppIdleT0        ; when the slide last changed or the mouse was last on it (ms)
Var AppCur0          ; where the mouse was when the slides showed ("x,y")
Var AppMouseSeen     ; 1 = the mouse has moved since then (only then does it hold the slides)
Var AppSlideCtl
Var AppFailName      ; the file AppCanWrite couldn't write
; [APP] The slide change: the new slide comes in sideways while it fades in over the one that
; showed, which moves off the other way (AppSlideDraw). Each step is drawn into $AppOut and put
; on the screen in one go (never through the picture control's own painting, which showed a white
; flash while Windows was busy); $AppFrame keeps the last step for the control's own repaints.
Var AppFrame
Var AppFrom
Var AppTo
Var AppOut
Var AppMix
Var AppDcF           ; picks up a picture shared with a control, for one copy at a time
Var AppDcFrom
Var AppDcTo
Var AppDcOut
Var AppDcMix
Var AppDir           ; 1 = the next slide (comes from the right), -1 = the previous one
Var AppIntro         ; 1 = the first slide still has to come in (once, when Setup's window shows)
Var AppFadeT0
Var AppSW            ; the slide's size, the moving part's height and how far it moves, on screen
Var AppSH
Var AppSTop
Var AppShift
; pictures and buttons
Var AppBmpLogo
Var AppBmpOn
Var AppBmpOff
Var AppBmpField
Var AppB1Ctl
Var AppB1N
Var AppB1H
Var AppB1X
Var AppB1Now         ; n, h or x
Var AppB2Ctl
Var AppB2N
Var AppB2H
Var AppB2Now
; the options
Var AppMode          ; install, update (older installed), same, older (newer installed)
Var AppOldVersion
Var AppCmp
Var AppR6
Var AppUiChoice      ; read by PageLeaveReinstall instead of its old radio buttons
Var AppDesktop
Var AppStart
Var AppBrowsers
Var AppSwDesk
Var AppSwStart
Var AppSwBrow
Var AppPathCtl
Var AppNoteCtl
Var AppHeld          ; 1 = Install can't be clicked (the folder won't do)
Var AppRunning       ; 1 = ${APP_NAME} is open (said in the window)
Var AppAgreedClose   ; 1 = ${APP_NAME} may be closed without asking again (it was said)
Var AppPassiveKeep
Var AppOwnCheck      ; 1 = AppCheckRunning uses its own words (see there)
Var AppPassiveOrig
Var AppOpen          ; done page: 1 = start ${APP_NAME} on the way out
Var AppFolderMsg
Var AppOldDir        ; where the installed ${APP_NAME} is (not "install")
Var AppCanMove       ; 1 = the installed ${APP_NAME}'s folder won't do: Change moves ${APP_NAME}
Var AppMoved         ; 1 = ${APP_NAME} goes to another folder than the installed one's
Var AppChangeCtl
; The keyboard's and screen readers' stand-ins (AppProxy) for what is drawn as pictures.
Var AppPxChange
Var AppPxB1
Var AppPxB2
Var AppPxDesk
Var AppPxStart
Var AppPxBrow
Var AppPxTerms
Var AppPxPrev        ; the slides' ‹ and ›, and the line beside them (they stay on a language switch)
Var AppPxNext
Var AppSepCtl
; [APP] The language choice (top right of the main page): its two words and their stand-ins
; (radio buttons: Tab reaches the one chosen, the arrows move between them, Narrator reads
; "English, radio button, selected").
!if ${APP_FRENCH} == 1
Var AppLangEnCtl
Var AppLangFrCtl
Var AppPxEn
Var AppPxFr
Var AppLangNext      ; the language just clicked (applied a moment later: AppLangApply)
Var AppLangFocus     ; which control had the keyboard when it was clicked
Var AppLangPicked    ; 1 = a language was picked in the window (even the one shown)
!endif
; [APP] Uninstaller: 1 = "Also delete Private mode's files" (never in a silent or passive run).
Var AppDeletePrivate
; The focus ring (4 thin lines), shown around what the keyboard is on.
Var AppRing1
Var AppRing2
Var AppRing3
Var AppRing4
Var AppRingOn
; The progress page: 1 = ${APP_NAME}'s look shows it; its Close (shown only when something went wrong).
Var AppInFiles
Var AppFailBtn
Var AppFailPic
Var AppFailBmp
Var AppChoicesOk     ; 0 = ${APP_EXE} couldn't take the window's choices (said when done)
; Arguments and result of the helper functions below.
Var AppA1
Var AppA2
Var AppA3
Var AppRet

; ---------- scaled sizes ----------
; Every size in this file is in pixels at 100 % (96 dpi); these make them the screen's.
!macro AppScale OUT VAL
  IntOp ${OUT} ${VAL} * $AppDpi
  IntOp ${OUT} ${OUT} + 48
  IntOp ${OUT} ${OUT} / 96
!macroend

; Draws the slide in SRC into DC, moved sideways by X (the strip it leaves gets the background),
; and the dots and arrows under it where they are.
!macro AppShifted DC SRC X
  System::Call "gdi32::BitBlt(p ${DC}, i ${X}, i 0, i $AppSW, i $AppSTop, p ${SRC}, i 0, i 0, i 0x00CC0020)"
  ${If} ${X} < 0
    IntOp $6 $AppSW + ${X}
    IntOp $5 0 - ${X}
    System::Call "gdi32::PatBlt(p ${DC}, i r6, i 0, i r5, i $AppSTop, i 0x00F00021)"
  ${ElseIf} ${X} > 0
    System::Call "gdi32::PatBlt(p ${DC}, i 0, i 0, i ${X}, i $AppSTop, i 0x00F00021)"
  ${EndIf}
  IntOp $5 $AppSH - $AppSTop
  System::Call "gdi32::BitBlt(p ${DC}, i 0, i $AppSTop, i $AppSW, i r5, p ${SRC}, i 0, i $AppSTop, i 0x00CC0020)"
!macroend

; A control on the current nsDialogs page: VAR = its handle.
!macro AppCtl VAR CLASS STYLE X Y W H TEXT
  !insertmacro AppScale $AppX ${X}
  !insertmacro AppScale $AppY ${Y}
  !insertmacro AppScale $AppW ${W}
  !insertmacro AppScale $AppH ${H}
  nsDialogs::CreateControl ${CLASS} ${STYLE} 0 $AppX $AppY $AppW $AppH "${TEXT}"
  Pop ${VAR}
!macroend

; A control on a page that isn't nsDialogs' (the progress pages): PARENT is that page.
!macro AppRaw VAR PARENT CLASS STYLE X Y W H TEXT
  !insertmacro AppScale $AppX ${X}
  !insertmacro AppScale $AppY ${Y}
  !insertmacro AppScale $AppW ${W}
  !insertmacro AppScale $AppH ${H}
  ; (Texts go to System::Call in a register: a comma in one would split its arguments.)
  Push $R9
  StrCpy $R9 "${TEXT}"
  System::Call 'user32::CreateWindowExW(i 0, w "${CLASS}", w R9, i ${STYLE}, i $AppX, i $AppY, i $AppW, i $AppH, p ${PARENT}, p 0, p 0, p 0) p .s'
  Exch
  Pop $R9
  Pop ${VAR}
!macroend

; A label: FONT, text colour, background colour.
!macro AppLabel VAR X Y W H STYLE FONT FG BG TEXT
  !insertmacro AppCtl ${VAR} STATIC ${STYLE} ${X} ${Y} ${W} ${H} "${TEXT}"
  SendMessage ${VAR} ${WM_SETFONT} ${FONT} 1
  !insertmacro AppColors ${VAR} ${FG} ${BG}
!macroend

; ${APP_NAME}'s colours, by name, in the look Setup uses. SetCtlColors takes its colours when Setup is
; built, so each look's are written out here (the same as scripts/installer-art.mjs).
!define APP_D_BG 0x14171b
!define APP_D_PANE 0x0f1114
!define APP_D_TEXT 0xe9eaee
!define APP_D_MUT 0x8b919c
!define APP_D_LN 0x262a31
!define APP_D_RA 0x1b1f24
!define APP_D_WARN 0xff7a7b
; [APP] The dark look's accent: APP_ACCENT_DARK (template 1.1.1; hooks.nsh makes it APP_ACCENT when
; app.nsh leaves it out), for an app whose colour is too dark to read there (Dobo: ink on paper,
; paper on dark). The art config's accentDark says the same.
!define APP_D_ACCENT ${APP_ACCENT_DARK}
!define APP_L_BG 0xffffff
!define APP_L_PANE 0xf6f7f9
!define APP_L_TEXT 0x17181c
!define APP_L_MUT 0x6b7280
!define APP_L_LN 0xe2e7ee
!define APP_L_RA 0xf3f5f8
!define APP_L_WARN 0xd33a3d
!define APP_L_ACCENT ${APP_ACCENT}
!macro AppColors HWND FG BG
  ${If} $AppLook == "d"
    SetCtlColors ${HWND} ${APP_D_${FG}} ${APP_D_${BG}}
  ${Else}
    SetCtlColors ${HWND} ${APP_L_${FG}} ${APP_L_${BG}}
  ${EndIf}
!macroend

!macro AppPicture VAR X Y W H BMP
  !insertmacro AppCtl ${VAR} STATIC ${APP_PIC} ${X} ${Y} ${W} ${H} ""
  SendMessage ${VAR} ${STM_SETIMAGE} 0 ${BMP}
!macroend

; What is drawn as a picture (buttons, switches, links, the slides' arrows) gets a real button
; under it, the same size, for the keyboard and screen readers: Tab reaches it, Narrator reads
; TEXT (and a switch's on/off), Space or Enter clicks it. It is never seen (the picture covers it
; and WS_CLIPSIBLINGS keeps it from painting over it); the focus ring shows where Tab is. The
; mouse still clicks the picture. Tab goes in the order they are made (each goes to the bottom
; of the window's order, under the one before).
!macro AppProxy VAR STYLE X Y W H TEXT
  !insertmacro AppCtl ${VAR} BUTTON ${STYLE} ${X} ${Y} ${W} ${H} "${TEXT}"
  SendMessage ${VAR} ${WM_SETFONT} $AppFSmall 1
  System::Call "user32::SetWindowPos(p ${VAR}, p 1, i 0, i 0, i 0, i 0, i 0x13)"
!macroend

; The focus ring's 4 lines, hidden until Tab is used (AppFocusTick places them).
!macro AppRingMake
  !insertmacro AppLabel $AppRing1 0 0 1 1 ${APP_HIDDEN_LABEL} $AppFBody TEXT TEXT ""
  !insertmacro AppLabel $AppRing2 0 0 1 1 ${APP_HIDDEN_LABEL} $AppFBody TEXT TEXT ""
  !insertmacro AppLabel $AppRing3 0 0 1 1 ${APP_HIDDEN_LABEL} $AppFBody TEXT TEXT ""
  !insertmacro AppLabel $AppRing4 0 0 1 1 ${APP_HIDDEN_LABEL} $AppFBody TEXT TEXT ""
  StrCpy $AppRingOn ""
!macroend

!macro AppRingNone
  StrCpy $AppRing1 0
  StrCpy $AppRing2 0
  StrCpy $AppRing3 0
  StrCpy $AppRing4 0
  StrCpy $AppRingOn ""
!macroend

; Something went wrong while installing or uninstalling: the line under the bar says what (in
; the warning colour) and Close shows, so the window can be left (Windows' Cancel under ${APP_NAME}'s
; picture; Esc and the window's ✕ close it too). Only with ${APP_NAME}'s look on the progress page.
!macro AppFail TEXT
  ${If} $AppInFiles = 1
    Push $0
    Push $1
    FindWindow $0 "#32770" "" $HWNDPARENT
    GetDlgItem $1 $0 1006
    ; Windows' line under the bar doesn't wrap (SS_LEFTNOWORDWRAP): what went wrong may need two
    ; lines, so here it wraps (SS_LEFT) in a box tall enough for three (same place and width).
    Push $2
    Push $3
    System::Call "user32::GetWindowLongW(p r1, i -16) i .r2"
    IntOp $2 $2 & 0xFFFFFFE0
    System::Call "user32::SetWindowLongW(p r1, i -16, i r2)"
    System::Call "*(i 0, i 0, i 0, i 0) p .r3"
    System::Call "user32::GetClientRect(p r1, p r3)"
    System::Call "*$3(i, i, i .r2, i)"
    System::Free $3
    !insertmacro AppScale $3 52
    System::Call "user32::SetWindowPos(p r1, p 0, i 0, i 0, i r2, i r3, i 0x16)" ; NOMOVE NOZORDER NOACTIVATE
    Pop $3
    Pop $2
    SendMessage $1 ${WM_SETTEXT} 0 "STR:${TEXT}"
    !insertmacro AppColors $1 WARN BG
    System::Call "user32::InvalidateRect(p r1, p 0, i 1)"
    ShowWindow $AppFailBtn ${SW_SHOW}
    ShowWindow $AppFailPic ${SW_SHOW}
    Pop $1
    Pop $0
  ${EndIf}
!macroend

; [APP] Before a file is copied over: can it be written? A file another program holds (an
; antivirus scan, a window left open) is tried again for 5 s; still held, the install stops
; there with what happened under the bar and Close (AppFail), instead of Windows' own
; Abort / Retry / Ignore box over a window with no button. NAME: the file's name, to say it.
!macro AppCanWrite FILE NAME
  Push $0
  Push $1
  StrCpy $1 0
  ${Do}
    ${IfNot} ${FileExists} "${FILE}"
      ${Break}
    ${EndIf}
    ; GENERIC_WRITE, no sharing, OPEN_EXISTING: what copying over it needs.
    System::Call 'kernel32::CreateFileW(w "${FILE}", i 0x40000000, i 0, p 0, i 3, i 0x80, p 0) i .r0'
    ${If} $0 <> -1
      System::Call 'kernel32::CloseHandle(i r0)'
      ${Break}
    ${EndIf}
    IntOp $1 $1 + 1
    ${If} $1 >= 10
      StrCpy $AppFailName "${NAME}"
      Pop $1
      Pop $0
      !insertmacro AppT $AppTx appFailFile
      !insertmacro AppFail $AppTx
      Abort $AppTx
    ${EndIf}
    Sleep 500
  ${Loop}
  Pop $1
  Pop $0
!macroend

; ${APP_NAME}'s own Check: the same as Tauri's (asks before closing a ${APP_NAME} that is open), without the
; question when the window already said ${APP_NAME} will be closed. On the progress page in ${APP_NAME}'s look
; it is Tauri's check written out, so that a ${APP_NAME} that couldn't be closed (or "Cancel" to the
; question) says so under the bar, with Close (AppFail), instead of a frozen window.
!macro AppCheckRunning EXE
  StrCpy $AppPassiveKeep $PassiveMode
  ${If} $AppAgreedClose = 1
    StrCpy $PassiveMode 1
  ${EndIf}
  ; [APP] With French on, also when the window's main page is left (PageLeaveReinstall), so that
  ; its question is in the language shown (Tauri's check reads its LangStrings: the language
  ; Setup started in). Silent and passive runs keep Tauri's.
  StrCpy $AppOwnCheck $AppInFiles
  !if ${APP_FRENCH} == 1
  ${IfNot} ${Silent}
  ${AndIf} $AppPassiveOrig <> 1
    StrCpy $AppOwnCheck 1
  ${EndIf}
  !endif
  ${If} $AppOwnCheck = 1
    ; (A function: this macro is also used before the language files are included.)
    !ifdef __UNINSTALL__
      Call un.AppRunTexts
    !else
      Call AppRunTexts
    !endif
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 != ""
      !insertmacro RestartManager_RegisterFile $R0 "${EXE}"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        ${If} $0 = ${ERROR_MORE_DATA}
          StrCpy $1 1
          ${If} $PassiveMode != 1
            MessageBox MB_OKCANCEL $R2 IDOK +2
            StrCpy $1 0
          ${EndIf}
          ${If} $1 = 0
            !insertmacro RestartManager_EndSession $R0
            StrCpy $PassiveMode $AppPassiveKeep
            !insertmacro AppFail $R1
            Abort $R1
          ${EndIf}
          System::Call 'RSTRTMGR::RmShutdown(p R0, i ${RmForceShutdown}, p 0) i .r0'
          ${If} $0 <> 0
            !insertmacro RestartManager_EndSession $R0
            StrCpy $PassiveMode $AppPassiveKeep
            !insertmacro AppFail $R3
            Abort $R3
          ${EndIf}
        ${EndIf}
      ${EndIf}
      !insertmacro RestartManager_EndSession $R0
    ${EndIf}
  ${Else}
    !insertmacro CheckIfAppIsRunning "${EXE}" "${PRODUCTNAME}"
  ${EndIf}
  StrCpy $PassiveMode $AppPassiveKeep
!macroend

; ---------- the functions (installer and uninstaller each get their own copy) ----------
!macro APP_UI_COMMON UN

; [APP] AppCheckRunning's three messages, in the language shown -> $R1, $R2, $R3.
Function ${UN}AppRunTexts
  !insertmacro AppT $AppTx appRunning
  nsis_tauri_utils::StrReplace "$AppTx" "{{product_name}}" "${PRODUCTNAME}"
  Pop $R1
  !insertmacro AppT $AppTx appRunningOkKill
  nsis_tauri_utils::StrReplace "$AppTx" "{{product_name}}" "${PRODUCTNAME}"
  Pop $R2
  !insertmacro AppT $AppTx failedToKillApp
  nsis_tauri_utils::StrReplace "$AppTx" "{{product_name}}" "${PRODUCTNAME}"
  Pop $R3
FunctionEnd

; Colours, fonts and the screen's scale; GDI+ for the pictures.
Function ${UN}AppLookInit
  Push $0
  Push $1
  Push $2
  StrCpy $AppDpi 96
  System::Call "user32::GetDpiForWindow(p $HWNDPARENT) i .r0"
  ${If} $0 > 0
    StrCpy $AppDpi $0
  ${EndIf}
  ; Windows' "app mode": AppsUseLightTheme 0 = dark (missing = light, Windows' default).
  StrCpy $AppLook "l"
  ClearErrors
  ReadRegDWORD $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Themes\Personalize" "AppsUseLightTheme"
  ${IfNot} ${Errors}
  ${AndIf} $0 = 0
    StrCpy $AppLook "d"
  ${EndIf}
  ; For trying both looks (tests, screenshots).
  ReadEnvStr $0 APP_SETUP_LOOK
  ${If} $0 == "dark"
    StrCpy $AppLook "d"
  ${ElseIf} $0 == "light"
    StrCpy $AppLook "l"
  ${EndIf}
  ; ${APP_NAME}'s colours as numbers, for the title bar and the progress bar.
  ${If} $AppLook == "d"
    StrCpy $AppCBg ${APP_D_BG}
    StrCpy $AppCText ${APP_D_TEXT}
    StrCpy $AppCRa ${APP_D_RA}
  ${Else}
    StrCpy $AppCBg ${APP_L_BG}
    StrCpy $AppCText ${APP_L_TEXT}
    StrCpy $AppCRa ${APP_L_RA}
  ${EndIf}
  ; Fonts in pixels for this screen (negative = the letters' own height).
  !insertmacro AppScale $0 13
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $AppFBody $1
  !insertmacro AppScale $0 12
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $AppFSmall $1
  !insertmacro AppScale $0 11
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 400, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $AppFTiny $1
  !insertmacro AppScale $0 17
  IntOp $0 0 - $0
  System::Call 'gdi32::CreateFontW(i r0, i 0, i 0, i 0, i 600, i 0, i 0, i 0, i 1, i 0, i 0, i 5, i 0, w "Segoe UI") p .r1'
  StrCpy $AppFTitle $1
  ; GDI+ reads the PNG and JPEG pictures and scales them smoothly.
  System::Call "*(i 1, p 0, i 0, i 0) p .r0"
  System::Call "gdiplus::GdiplusStartup(*p .r1, p r0, p 0) i .r2"
  System::Free $0
  StrCpy $AppGdip $1
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $AppA1 = file name in art (with -d/-l and the extension), $AppA2 x $AppA3 = its size at
; 100 % -> $AppRet = a bitmap of that size for this screen, or 0.
Function ${UN}AppImage
  Push $0
  Push $1
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  Push $9
  !insertmacro AppScale $1 $AppA2
  !insertmacro AppScale $0 $AppA3
  StrCpy $9 0
  StrCpy $8 "$PLUGINSDIR\art\$AppA1"
  System::Call "gdiplus::GdipCreateBitmapFromFile(w r8, *p .r3) i .r8"
  ${If} $8 = 0
    System::Call "gdiplus::GdipGetImageWidth(p r3, *i .r6)"
    System::Call "gdiplus::GdipGetImageHeight(p r3, *i .r7)"
    System::Call "gdiplus::GdipCreateBitmapFromScan0(i r1, i r0, i 0, i 0x26200A, p 0, *p .r4) i .r8"
    System::Call "gdiplus::GdipGetImageGraphicsContext(p r4, *p .r5) i .r8"
    System::Call "gdiplus::GdipSetInterpolationMode(p r5, i 7)"   ; high quality bicubic
    System::Call "gdiplus::GdipSetPixelOffsetMode(p r5, i 2)"     ; high quality
    System::Call "gdiplus::GdipSetCompositingQuality(p r5, i 2)"
    ; Edges: the picture's own edge pixels, not a darker blend with nothing.
    System::Call "gdiplus::GdipCreateImageAttributes(*p .r8)"
    System::Call "gdiplus::GdipSetImageAttributesWrapMode(p r8, i 3, i 0, i 0)"
    System::Call "gdiplus::GdipDrawImageRectRectI(p r5, p r3, i 0, i 0, i r1, i r0, i 0, i 0, i r6, i r7, i 2, p r8, p 0, p 0)"
    System::Call "gdiplus::GdipDisposeImageAttributes(p r8)"
    System::Call "gdiplus::GdipDeleteGraphics(p r5)"
    System::Call "gdiplus::GdipCreateHBITMAPFromBitmap(p r4, *p .r9, i 0xff000000)"
    System::Call "gdiplus::GdipDisposeImage(p r4)"
    System::Call "gdiplus::GdipDisposeImage(p r3)"
    ; Made 24-bit: a picture control (comctl32 6) keeps its own copy of every 32-bit picture it
    ; is given, which would never be freed (the slides change every 4.5 s, the buttons on hover).
    ${If} $9 != 0
      Push $2
      System::Call "user32::GetDC(p 0) p .r2"
      System::Call "gdi32::CreateCompatibleDC(p r2) p .r5"
      System::Call "gdi32::CreateCompatibleDC(p r2) p .r6"
      System::Call "user32::ReleaseDC(p 0, p r2)"
      IntOp $7 0 - $0
      System::Call "*(i 40, i r1, i r7, &i2 1, &i2 24, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r8"
      System::Call "gdi32::CreateDIBSection(p r6, p r8, i 0, *p .r3, p 0, i 0) p .r4"
      System::Free $8
      ${If} $4 != 0
        System::Call "gdi32::SelectObject(p r5, p r9) p .r7"
        System::Call "gdi32::SelectObject(p r6, p r4) p .r8"
        System::Call "gdi32::BitBlt(p r6, i 0, i 0, i r1, i r0, p r5, i 0, i 0, i 0x00CC0020)"
        System::Call "gdi32::SelectObject(p r5, p r7)"
        System::Call "gdi32::SelectObject(p r6, p r8)"
        System::Call "gdi32::DeleteObject(p r9)"
        StrCpy $9 $4
      ${EndIf}
      System::Call "gdi32::DeleteDC(p r5)"
      System::Call "gdi32::DeleteDC(p r6)"
      Pop $2
    ${EndIf}
  ${EndIf}
  StrCpy $AppRet $9
  Pop $9
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $1
  Pop $0
FunctionEnd

; $AppA1 x $AppA2 (at 100 %): makes the window's inside that size, with only ${APP_NAME}'s pages
; in it (Tauri's header, buttons and lines hidden; Next and Cancel still work from the keyboard
; and from ${APP_NAME}'s own buttons), centred on its screen, with a title bar in ${APP_NAME}'s colours.
Function ${UN}AppWindow
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  !insertmacro AppScale $0 $AppA1
  !insertmacro AppScale $1 $AppA2
  ; The whole window for that inside.
  System::Call "user32::GetWindowLongW(p $HWNDPARENT, i -16) i .r6"
  System::Call "user32::GetWindowLongW(p $HWNDPARENT, i -20) i .r7"
  System::Call "*(i 0, i 0, i r0, i r1) p .r2"
  System::Call "user32::AdjustWindowRectExForDpi(p r2, i r6, i 0, i r7, i $AppDpi) i .r8"
  ${If} $8 = 0
    System::Call "user32::AdjustWindowRectEx(p r2, i r6, i 0, i r7)"
  ${EndIf}
  System::Call "*$2(i .r3, i .r4, i .r5, i .r6)"
  System::Free $2
  IntOp $5 $5 - $3
  IntOp $6 $6 - $4
  ; Centred in the work area of the screen the window is on.
  System::Call "user32::MonitorFromWindow(p $HWNDPARENT, i 2) p .r7"
  System::Call "*(i 40, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r2"
  System::Call "user32::GetMonitorInfoW(p r7, p r2)"
  System::Call "*$2(i, i, i, i, i, i .r3, i .r4, i .r7, i .r8)"
  System::Free $2
  IntOp $7 $7 - $3
  IntOp $7 $7 - $5
  IntOp $7 $7 / 2
  ; Bigger than the screen (a small screen at a high scale): its top left corner stays on it.
  ${IfThen} $7 < 0 ${|} StrCpy $7 0 ${|}
  IntOp $3 $3 + $7
  IntOp $8 $8 - $4
  IntOp $8 $8 - $6
  IntOp $8 $8 / 2
  ${IfThen} $8 < 0 ${|} StrCpy $8 0 ${|}
  IntOp $4 $4 + $8
  System::Call "user32::SetWindowPos(p $HWNDPARENT, p 0, i r3, i r4, i r5, i r6, i 0x14)"
  ; Tauri's own parts of the window, hidden (the buttons keep working).
  !insertmacro AppHideItem 1
  !insertmacro AppHideItem 2
  !insertmacro AppHideItem 3
  !insertmacro AppHideItem 1028
  !insertmacro AppHideItem 1256
  !insertmacro AppHideItem 1034
  !insertmacro AppHideItem 1035
  !insertmacro AppHideItem 1036
  !insertmacro AppHideItem 1037
  !insertmacro AppHideItem 1038
  !insertmacro AppHideItem 1039
  !insertmacro AppHideItem 1044
  !insertmacro AppHideItem 1045
  ; Pages fill the whole inside.
  GetDlgItem $2 $HWNDPARENT 1018
  System::Call "user32::SetWindowPos(p r2, p 0, i 0, i 0, i r0, i r1, i 0x14)"
  ; The title bar in ${APP_NAME}'s colours (Windows 11; Windows 10 shows its own).
  ${If} $AppLook == "d"
    System::Call "*(i 1) p .r2"
  ${Else}
    System::Call "*(i 0) p .r2"
  ${EndIf}
  System::Call "dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 20, p r2, i 4)"
  System::Free $2
  ; COLORREF is 0x00BBGGRR.
  StrCpy $AppA1 $AppCBg
  Call ${UN}AppColorRef
  System::Call "*(i $AppRet) p .r2"
  System::Call "dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 35, p r2, i 4)"
  System::Free $2
  StrCpy $AppA1 $AppCText
  Call ${UN}AppColorRef
  System::Call "*(i $AppRet) p .r2"
  System::Call "dwmapi::DwmSetWindowAttribute(p $HWNDPARENT, i 36, p r2, i 4)"
  System::Free $2
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $AppA1 = 0xRRGGBB -> $AppRet = 0x00BBGGRR (a Windows COLORREF).
Function ${UN}AppColorRef
  Push $1
  Push $2
  IntOp $1 $AppA1 & 0xff
  IntOp $1 $1 << 16
  IntOp $2 $AppA1 & 0xff00
  IntOp $1 $1 | $2
  IntOp $2 $AppA1 >> 16
  IntOp $2 $2 & 0xff
  IntOp $AppRet $1 | $2
  Pop $2
  Pop $1
FunctionEnd

; $AppA1 = a control -> $AppRet = 1 if the mouse is on it, else 0.
Function ${UN}AppMouseOn
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  StrCpy $AppRet 0
  ${If} $AppA1 <> 0
    System::Call "*(i 0, i 0) p .r1"
    System::Call "user32::GetCursorPos(p r1)"
    System::Call "*$1(i .r2, i .r3)"
    System::Free $1
    System::Call "*(i 0, i 0, i 0, i 0) p .r1"
    System::Call "user32::GetWindowRect(p $AppA1, p r1)"
    System::Call "*$1(i .r4, i .r5, i .r6, i .r7)"
    System::Free $1
    ${If} $2 >= $4
    ${AndIf} $2 < $6
    ${AndIf} $3 >= $5
    ${AndIf} $3 < $7
      ; [APP] And it's really Setup's window under the mouse: with another window over Setup
      ; (or Setup behind it), the mouse resting at that place on the screen doesn't count. It
      ; once kept the slides from ever moving on.
      System::Int64Op $3 << 32
      Pop $1
      System::Int64Op $2 & 0xFFFFFFFF
      Pop $4
      System::Int64Op $1 | $4
      Pop $1
      System::Call "user32::WindowFromPoint(l r1) p .r4"
      System::Call "user32::GetAncestor(p r4, i 2) p .r4" ; GA_ROOT
      ${If} $4 = $HWNDPARENT
        StrCpy $AppRet 1
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
FunctionEnd

; The page's buttons: the picture for their state (n, h(over), x = can't be clicked).
Function ${UN}AppButtonsTick
  Push $0
  Push $1
  ${If} $AppB1Ctl <> 0
    StrCpy $1 "n"
    ${If} $AppHeld = 1
    ${AndIf} $AppB1X <> 0
      StrCpy $1 "x"
    ${Else}
      StrCpy $AppA1 $AppB1Ctl
      Call ${UN}AppMouseOn
      ${IfThen} $AppRet = 1 ${|} StrCpy $1 "h" ${|}
    ${EndIf}
    ${If} $1 != $AppB1Now
      StrCpy $AppB1Now $1
      ${If} $1 == "h"
        SendMessage $AppB1Ctl ${STM_SETIMAGE} 0 $AppB1H
      ${ElseIf} $1 == "x"
        SendMessage $AppB1Ctl ${STM_SETIMAGE} 0 $AppB1X
      ${Else}
        SendMessage $AppB1Ctl ${STM_SETIMAGE} 0 $AppB1N
      ${EndIf}
    ${EndIf}
  ${EndIf}
  ${If} $AppB2Ctl <> 0
    StrCpy $AppA1 $AppB2Ctl
    Call ${UN}AppMouseOn
    StrCpy $1 "n"
    ${IfThen} $AppRet = 1 ${|} StrCpy $1 "h" ${|}
    ${If} $1 != $AppB2Now
      StrCpy $AppB2Now $1
      ${If} $1 == "h"
        SendMessage $AppB2Ctl ${STM_SETIMAGE} 0 $AppB2H
      ${Else}
        SendMessage $AppB2Ctl ${STM_SETIMAGE} 0 $AppB2N
      ${EndIf}
    ${EndIf}
  ${EndIf}
  Call ${UN}AppFocusTick
  Pop $1
  Pop $0
FunctionEnd

; The focus ring: around the stand-in (AppProxy) the keyboard is on, 3 px out, 2 px thick.
; Only once a key was used (Windows hides focus until then, as in its own windows).
Function ${UN}AppFocusTick
  ${If} $AppRing1 = 0
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  System::Call "user32::GetFocus() p .r0"
  System::Call "user32::GetParent(p r0) p .r1"
  ${If} $1 != $AppDialog
    StrCpy $0 0
  ${EndIf}
  SendMessage $AppDialog 0x129 0 0 $1 ; WM_QUERYUISTATE
  IntOp $1 $1 & 1                      ; UISF_HIDEFOCUS
  ${IfThen} $1 <> 0 ${|} StrCpy $0 0 ${|}
  ${If} $0 != $AppRingOn
    StrCpy $AppRingOn $0
    ${If} $0 = 0
      ShowWindow $AppRing1 ${SW_HIDE}
      ShowWindow $AppRing2 ${SW_HIDE}
      ShowWindow $AppRing3 ${SW_HIDE}
      ShowWindow $AppRing4 ${SW_HIDE}
    ${Else}
      System::Call "*(i 0, i 0, i 0, i 0) p .r1"
      System::Call "user32::GetWindowRect(p r0, p r1)"
      System::Call "user32::MapWindowPoints(p 0, p $AppDialog, p r1, i 2)"
      System::Call "*$1(i .r2, i .r3, i .r4, i .r5)"
      System::Free $1
      !insertmacro AppScale $6 3 ; gap
      !insertmacro AppScale $7 2 ; thickness
      ; Outer box: left $2, top $3, right $4, bottom $5.
      IntOp $2 $2 - $6
      IntOp $2 $2 - $7
      IntOp $3 $3 - $6
      IntOp $3 $3 - $7
      IntOp $4 $4 + $6
      IntOp $4 $4 + $7
      IntOp $5 $5 + $6
      IntOp $5 $5 + $7
      IntOp $8 $4 - $2 ; width
      IntOp $1 $5 - $3 ; height
      System::Call "user32::SetWindowPos(p $AppRing1, p 0, i r2, i r3, i r8, i r7, i 0x50)"
      IntOp $6 $5 - $7
      System::Call "user32::SetWindowPos(p $AppRing2, p 0, i r2, i r6, i r8, i r7, i 0x50)"
      System::Call "user32::SetWindowPos(p $AppRing3, p 0, i r2, i r3, i r7, i r1, i 0x50)"
      IntOp $6 $4 - $7
      System::Call "user32::SetWindowPos(p $AppRing4, p 0, i r6, i r3, i r7, i r1, i 0x50)"
    ${EndIf}
  ${EndIf}
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; $AppA1 = a button's name (art/b-<name>-<state>-<look>[-fr].png: its words are in the picture,
; in the language shown), $AppA2 = its width at 100 % -> its pictures in $AppB1N/H/X
; (AppLoadButton2: $AppB2N/H).
Function ${UN}AppLoadButton1
  Push $0
  Push $1
  StrCpy $0 $AppA1
  StrCpy $1 $AppA2
  StrCpy $AppA3 40
  StrCpy $AppA1 "b-$0-n-$AppLook$AppArtSfx.png"
  Call ${UN}AppImage
  StrCpy $AppB1N $AppRet
  StrCpy $AppA1 "b-$0-h-$AppLook$AppArtSfx.png"
  StrCpy $AppA2 $1
  Call ${UN}AppImage
  StrCpy $AppB1H $AppRet
  StrCpy $AppB1X 0
  ${If} ${FileExists} "$PLUGINSDIR\art\b-$0-x-$AppLook$AppArtSfx.png"
    StrCpy $AppA1 "b-$0-x-$AppLook$AppArtSfx.png"
    StrCpy $AppA2 $1
    Call ${UN}AppImage
    StrCpy $AppB1X $AppRet
  ${EndIf}
  StrCpy $AppB1Now ""
  Pop $1
  Pop $0
FunctionEnd

Function ${UN}AppLoadButton2
  Push $0
  Push $1
  StrCpy $0 $AppA1
  StrCpy $1 $AppA2
  StrCpy $AppA3 40
  StrCpy $AppA1 "b-$0-n-$AppLook$AppArtSfx.png"
  Call ${UN}AppImage
  StrCpy $AppB2N $AppRet
  StrCpy $AppA1 "b-$0-h-$AppLook$AppArtSfx.png"
  StrCpy $AppA2 $1
  Call ${UN}AppImage
  StrCpy $AppB2H $AppRet
  StrCpy $AppB2Now ""
  Pop $1
  Pop $0
FunctionEnd

; $AppA1 = a font, $AppA2 = a text -> $AppRet = its width in pixels.
Function ${UN}AppTextWidth
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  ; (The text goes to System::Call in a register: a comma in it would split its arguments.)
  StrCpy $1 $AppA2
  System::Call "user32::GetDC(p 0) p .r2"
  System::Call "gdi32::SelectObject(p r2, p $AppA1) p .r3"
  StrLen $4 $1
  System::Call "*(i 0, i 0) p .r0"
  System::Call "gdi32::GetTextExtentPoint32W(p r2, w r1, i r4, p r0)"
  System::Call "*$0(i .r4, i)"
  System::Free $0
  System::Call "gdi32::SelectObject(p r2, p r3)"
  System::Call "user32::ReleaseDC(p 0, p r2)"
  StrCpy $AppRet $4
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; The look of a progress page (Tauri's, Windows' own): its bar and line of text moved and
; coloured, its list hidden. $0 = the page. X, W (at 100 %): where the bar goes; Y: its top.
Function ${UN}AppStyleProgress
  ; $0 page, $1 x, $2 y, $3 w (at 100 %)
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  GetDlgItem $4 $0 1016
  ShowWindow $4 ${SW_HIDE}
  GetDlgItem $4 $0 1027
  ShowWindow $4 ${SW_HIDE}
  ; The bar: flat, 6 px, ${APP_NAME}'s coral on the field colour.
  GetDlgItem $4 $0 1004
  System::Call 'uxtheme::SetWindowTheme(p r4, w " ", w " ")'
  System::Call "user32::GetWindowLongW(p r4, i -16) i .r5"
  IntOp $5 $5 & 0xFF7FFFFF ; no WS_BORDER
  IntOp $5 $5 | 0x1        ; PBS_SMOOTH
  System::Call "user32::SetWindowLongW(p r4, i -16, i r5)"
  System::Call "user32::SetWindowLongW(p r4, i -20, i 0)"
  StrCpy $AppA1 $AppCRa
  Call ${UN}AppColorRef
  SendMessage $4 0x2001 0 $AppRet ; PBM_SETBKCOLOR
  ; The look's own accent (APP_D_ACCENT / APP_L_ACCENT).
  ${If} $AppLook == "d"
    StrCpy $AppA1 ${APP_D_ACCENT}
  ${Else}
    StrCpy $AppA1 ${APP_L_ACCENT}
  ${EndIf}
  Call ${UN}AppColorRef
  SendMessage $4 0x409 0 $AppRet ; PBM_SETBARCOLOR
  !insertmacro AppScale $5 $1
  !insertmacro AppScale $6 $2
  !insertmacro AppScale $7 $3
  !insertmacro AppScale $8 6
  System::Call "user32::SetWindowPos(p r4, p 0, i r5, i r6, i r7, i r8, i 0x34)"
  ; The line under it ("Copying ${APP_EXE}... 40%", or what went wrong).
  GetDlgItem $4 $0 1006
  IntOp $6 $2 + 16
  !insertmacro AppScale $6 $6
  !insertmacro AppScale $8 36
  System::Call "user32::SetWindowPos(p r4, p 0, i r5, i r6, i r7, i r8, i 0x14)"
  SendMessage $4 ${WM_SETFONT} $AppFSmall 1
  !insertmacro AppColors $4 MUT BG
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
FunctionEnd

; $0 = a progress page (Windows' own): it fills the window ($AppA1 x $AppA2 at 100 %) and gets
; ${APP_NAME}'s background (a label under everything: that page doesn't take colours itself).
Function ${UN}AppPageFill
  Push $1
  Push $2
  !insertmacro AppScale $1 $AppA1
  !insertmacro AppScale $2 $AppA2
  System::Call "user32::SetWindowPos(p r0, p 0, i 0, i 0, i r1, i r2, i 0x16)"
  ; (WS_CLIPSIBLINGS: it never paints over the bar and the words on it.)
  !insertmacro AppRaw $1 $0 STATIC 0x54000080 0 0 $AppA1 $AppA2 ""
  !insertmacro AppColors $1 BG BG
  System::Call "user32::SetWindowPos(p r1, p 1, i 0, i 0, i 0, i 0, i 0x13)" ; to the bottom
  Pop $2
  Pop $1
FunctionEnd

; The progress page's Close, ready and hidden ($0 = the page; $1, $2, $3 = x, y, w at 100 %;
; $AppFailBmp its picture): Windows' own Cancel, moved there, under ${APP_NAME}'s picture of Close,
; which the mouse goes through. Shown by AppFail when something goes wrong (Windows enables
; Cancel then; clicking it ends Setup, as do Esc and the window's ✕).
Function ${UN}AppFailClose
  Push $4
  Push $5
  ; The page paints around them, never over them.
  System::Call "user32::GetWindowLongW(p r0, i -16) i .r4"
  IntOp $4 $4 | 0x04000000 ; WS_CLIPSIBLINGS
  System::Call "user32::SetWindowLongW(p r0, i -16, i r4)"
  GetDlgItem $AppFailBtn $HWNDPARENT 2
  System::Call "user32::GetWindowLongW(p $AppFailBtn, i -16) i .r4"
  IntOp $4 $4 | 0x04000000
  System::Call "user32::SetWindowLongW(p $AppFailBtn, i -16, i r4)"
  !insertmacro AppT $AppTx appClose
  SendMessage $AppFailBtn ${WM_SETTEXT} 0 "STR:$AppTx"
  !insertmacro AppScale $4 $1
  !insertmacro AppScale $5 $2
  !insertmacro AppScale $AppW $3
  !insertmacro AppScale $AppH 40
  System::Call "user32::SetWindowPos(p $AppFailBtn, p 0, i r4, i r5, i $AppW, i $AppH, i 0x10)"
  !insertmacro AppRaw $AppFailPic $HWNDPARENT STATIC ${APP_PIC_PASS} $1 $2 $3 40 ""
  SendMessage $AppFailPic ${STM_SETIMAGE} 0 $AppFailBmp
  System::Call "user32::SetWindowPos(p $AppFailPic, p 0, i 0, i 0, i 0, i 0, i 0x13)"
  ; Windows shows its Back button again when something went wrong: none to see (no size).
  GetDlgItem $4 $HWNDPARENT 3
  System::Call "user32::SetWindowPos(p r4, p 0, i 0, i 0, i 0, i 0, i 0x16)"
  Pop $5
  Pop $4
FunctionEnd

; Tauri's Back, Next and Cancel stay hidden on every page (Windows shows Back again on each one);
; they keep working from the keyboard and from ${APP_NAME}'s own buttons.
Function ${UN}AppHideNav
  Push $2
  !insertmacro AppHideItem 1
  !insertmacro AppHideItem 2
  !insertmacro AppHideItem 3
  ; The focus ring waits for the keyboard on each page (Tab shows it, as in Windows' own
  ; windows): WM_CHANGEUISTATE, UIS_SET UISF_HIDEFOCUS.
  SendMessage $HWNDPARENT 0x127 0x10001 0
  Pop $2
FunctionEnd

!macroend

!macro AppHideItem ID
  GetDlgItem $2 $HWNDPARENT ${ID}
  ShowWindow $2 ${SW_HIDE}
!macroend

; One slide's picture: N (1..5), SFX ("" or "-fr") -> VAR.
!macro AppSlideRead N SFX VAR
  StrCpy $AppA1 "slide${N}-$AppLook${SFX}.jpg"
  StrCpy $AppA2 530
  StrCpy $AppA3 540
  Call AppImage
  StrCpy ${VAR} $AppRet
!macroend

; ---------- the installer's pages ----------
!macro APP_UI_FUNCTIONS
!ifndef AppEn_appInstallIn
  !error 'English.nsh is from template 1.0.x (plain LangStrings): write each line as !insertmacro AppEn <name> "<text>" (see CHANGELOG, Template 1.1.0: Updating an app from 1.0.x).'
!endif
!insertmacro APP_UI_COMMON ""
!insertmacro APP_UI_COMMON "un."

!if ${APP_FRENCH} == 1
!ifndef APP_FRENCH_TEXTS
  !error 'APP_FRENCH is on but French.nsh is not included: tauri.conf.json needs "languages": ["English", "French"] and "customLanguageFiles": { "English": "installer/English.nsh", "French": "installer/French.nsh" }.'
!endif

; [APP] NSIS's own words that only read the language Setup started in (English.nsh: ^Extract),
; in the language shown.
Function AppLangTexts
  !insertmacro AppT $AppTxExtract appExtract
FunctionEnd

; [APP] Reads the next French slide (AppTick: one at a time while the window is idle, so the
; first switch to French shows its slides at once).
Function AppReadFrSlide
  ${If} $AppSFReady == ""
    StrCpy $AppSFReady 0
  ${EndIf}
  ${If} $AppSFReady >= ${APP_SLIDES}
    Return
  ${EndIf}
  IntOp $AppSFReady $AppSFReady + 1
  ${If} $AppSFReady = 1
    !insertmacro AppSlideRead 1 "-fr" $AppSF1
  ${ElseIf} $AppSFReady = 2
    !insertmacro AppSlideRead 2 "-fr" $AppSF2
  ${ElseIf} $AppSFReady = 3
    !insertmacro AppSlideRead 3 "-fr" $AppSF3
  ${ElseIf} $AppSFReady = 4
    !insertmacro AppSlideRead 4 "-fr" $AppSF4
  ${Else}
    !insertmacro AppSlideRead 5 "-fr" $AppSF5
  ${EndIf}
FunctionEnd
!endif

; [APP] installer.nsi (PageLeaveReinstall): the installed version's uninstaller failed.
Function AppUnableToUninstall
  !insertmacro AppT $AppTx unableToUninstall
  MessageBox MB_ICONEXCLAMATION $AppTx
FunctionEnd

; [APP] $AppS1..5 = the slides of the language shown.
Function AppUseSlides
  !if ${APP_FRENCH} == 1
  ${If} $AppLang == "fr"
    ${Do}
      ${IfThen} $AppSFReady >= ${APP_SLIDES} ${|} ${Break} ${|}
      Call AppReadFrSlide
    ${Loop}
    StrCpy $AppS1 $AppSF1
    StrCpy $AppS2 $AppSF2
    StrCpy $AppS3 $AppSF3
    StrCpy $AppS4 $AppSF4
    StrCpy $AppS5 $AppSF5
    Return
  ${EndIf}
  !endif
  StrCpy $AppS1 $AppSE1
  StrCpy $AppS2 $AppSE2
  StrCpy $AppS3 $AppSE3
  StrCpy $AppS4 $AppSE4
  StrCpy $AppS5 $AppSE5
FunctionEnd

; MUI_CUSTOMFUNCTION_GUIINIT (hooks.nsh): the window, before it shows.
Function AppGuiInit
  Call AppInstallDir
  StrCpy $AppPassiveOrig $PassiveMode
  ${If} $PassiveMode = 1
    Return
  ${EndIf}
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\art"
  File "${APP_ART}\*.jpg"
  File "${APP_ART}\*.png"
  ; [APP] Out of the pictures' folder again: SetOutPath also made it Setup's working folder, and
  ; Windows can't delete a working folder, so NSIS's own clean-up at exit left it behind, empty, in %TEMP%.
  SetOutPath "$TEMP"
  Call AppLookInit
  StrCpy $AppA1 860
  StrCpy $AppA2 540
  Call AppWindow
  Call AppDetect
  !if ${APP_FRENCH} == 1
  ; [APP] Over an installed ${APP_NAME} (update, reinstall, an older version): the window opens in
  ; the language saved for it ("Installer Language": the one Setup last ran in, which the
  ; installed app updates when its own language changes, README: The app's language). A first
  ; install opens in English. Either way the choice in the window still works, and only a pick
  ; counts as a pick (the app gets --lang only then, hooks.nsh).
  ${If} $AppMode != "install"
    ReadRegStr $AppTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
    ${If} $AppTx == ${LANG_FRENCH}
      StrCpy $AppLang "fr"
      StrCpy $LANGUAGE ${LANG_FRENCH}
      StrCpy $AppArtSfx "-fr"
      Call AppLangTexts
    ${EndIf}
  ${EndIf}
  !endif
  ; The slides, once for every page (English ones; the French ones are read while the window is
  ; idle, AppTick, or at once when it opens in French: AppUseSlides).
  !insertmacro AppSlideRead 1 "" $AppSE1
  !insertmacro AppSlideRead 2 "" $AppSE2
  !insertmacro AppSlideRead 3 "" $AppSE3
  !insertmacro AppSlideRead 4 "" $AppSE4
  !insertmacro AppSlideRead 5 "" $AppSE5
  Call AppUseSlides
  StrCpy $AppA1 "logo-$AppLook.png"
  StrCpy $AppA2 160
  StrCpy $AppA3 44
  Call AppImage
  StrCpy $AppBmpLogo $AppRet
  StrCpy $AppA1 "on-$AppLook.png"
  StrCpy $AppA2 32
  StrCpy $AppA3 18
  Call AppImage
  StrCpy $AppBmpOn $AppRet
  StrCpy $AppA1 "off-$AppLook.png"
  StrCpy $AppA2 32
  StrCpy $AppA3 18
  Call AppImage
  StrCpy $AppBmpOff $AppRet
  StrCpy $AppA1 "field-$AppLook.png"
  StrCpy $AppA2 282
  StrCpy $AppA3 32
  Call AppImage
  StrCpy $AppBmpField $AppRet
  StrCpy $AppSlide 1
  StrCpy $AppAuto 1
  StrCpy $AppDir 1
  StrCpy $AppIntro 1
FunctionEnd

; The window's title on every page (Windows sets "${APP_NAME} Setup" again on each one).
Function AppSetTitle
  ${If} $AppMode == "update"
    !insertmacro AppT $AppTx appUpdateTitle
    SendMessage $HWNDPARENT ${WM_SETTEXT} 0 "STR:$AppTx"
  !if ${APP_FRENCH} == 1
  ${Else}
    ; [APP] NSIS's own title ("<App> Setup") is in the language Setup started in.
    !insertmacro AppT $AppTx appSetupTitle
    SendMessage $HWNDPARENT ${WM_SETTEXT} 0 "STR:$AppTx"
  !endif
  ${EndIf}
FunctionEnd

; The line under the logo for what this Setup does -> $AppRet.
Function AppModeTag
  ${If} $AppMode == "update"
    !insertmacro AppT $AppRet appTagUpdate
  ${ElseIf} $AppMode == "same"
    !insertmacro AppT $AppRet appTagSame
  ${ElseIf} $AppMode == "older"
    !insertmacro AppT $AppRet appTagOlder
  ${Else}
    !insertmacro AppT $AppRet appTag
  ${EndIf}
FunctionEnd

; What is installed already (the same as Tauri's reinstall page worked out): $AppMode,
; $AppOldVersion, $AppCmp, and the switches as they are for it.
Function AppDetect
  Push $0
  Push $R0
  Push $R1
  Push $R6
  StrCpy $AppMode "install"
  StrCpy $AppOldVersion ""
  StrCpy $AppDesktop ${APP_DEFAULT_DESKTOP}
  StrCpy $AppStart ${APP_DEFAULT_START}
  StrCpy $AppBrowsers ${APP_DEFAULT_BROWSERS}
  Call FindWixInstall
  StrCpy $AppR6 $R6
  ${If} $R6 != ""
    StrCpy $WixMode 1
    ReadRegStr $AppOldVersion HKLM "$R6" "DisplayVersion"
  ${Else}
    ReadRegStr $R0 SHCTX "${UNINSTKEY}" ""
    ReadRegStr $R1 SHCTX "${UNINSTKEY}" "UninstallString"
    ${If} "$R0$R1" == ""
      ; [APP] A portable copy that already starts with Windows keeps doing so: Windows has one
      ; "${PRODUCTNAME}" start-up entry, and a new install switched on would take it over. Its
      ; entry is "<exe in quotes>" [args], or the exe unquoted then args (tauri-plugin-autostart
      ; writes `C:\...\App.exe --hidden`); one whose exe is gone doesn't count.
      ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
      StrCpy $R1 $0 1
      ${If} $R1 == '"'
        StrCpy $R0 1
        ${Do}
          StrCpy $R1 $0 1 $R0
          ${IfThen} $R1 == '"' ${|} ${Break} ${|}
          ${IfThen} $R1 == "" ${|} ${Break} ${|}
          IntOp $R0 $R0 + 1
        ${Loop}
        IntOp $R0 $R0 - 1
        StrCpy $R1 $0 $R0 1
        ${If} ${FileExists} "$R1"
          StrCpy $AppStart 0
        ${EndIf}
      ${ElseIf} $0 != ""
        ; Unquoted: the exe is everything up to the first ".exe" (from Dobo, 832fbaf). [APP] Dobo
        ; 1.0 wrote it so; 1.1 writes it quoted and quotes an old one at start (install.rs).
        StrCpy $R0 0
        ${Do}
          StrCpy $R1 $0 4 $R0
          ${IfThen} $R1 == ".exe" ${|} ${Break} ${|}
          ${IfThen} $R1 == "" ${|} ${Break} ${|}
          IntOp $R0 $R0 + 1
        ${Loop}
        ${If} $R1 == ".exe"
          IntOp $R0 $R0 + 4
          StrCpy $R1 $0 $R0
          ${If} ${FileExists} "$R1"
            StrCpy $AppStart 0
          ${EndIf}
        ${EndIf}
      ${EndIf}
      Goto app_detect_done
    ${EndIf}
    ReadRegStr $AppOldVersion SHCTX "${UNINSTKEY}" "DisplayVersion"
  ${EndIf}
  nsis_tauri_utils::SemverCompare "${VERSION}" $AppOldVersion
  Pop $AppCmp
  ${If} $AppCmp = 1
    StrCpy $AppMode "update"
  ${ElseIf} $AppCmp = 0
    StrCpy $AppMode "same"
  ${Else}
    StrCpy $AppMode "older"
  ${EndIf}
  ; Its folder (as Tauri's RestorePreviousInstallLocation reads it; $INSTDIR is another one when
  ; Setup was started with /D=).
  ReadRegStr $AppOldDir SHCTX "${MANUPRODUCTKEY}" ""
  ${IfThen} $AppOldDir == "" ${|} StrCpy $AppOldDir $INSTDIR ${|}
  ; The switches show what this ${APP_NAME} has now.
  !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  Pop $AppDesktop
  StrCpy $AppStart 0
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  ${StrCase} $0 $0 "L"
  ${StrCase} $R0 "$INSTDIR\${MAINBINARYNAME}.exe" "L"
  ${StrLoc} $R1 $0 $R0 ">"
  ${If} $R1 != ""
    StrCpy $AppStart 1
  ${EndIf}
  !if ${APP_BROWSER_SWITCH} == 1
  StrCpy $AppBrowsers 0
  ClearErrors
  ReadRegStr $0 HKCU "Software\Google\Chrome\NativeMessagingHosts\${APP_NATIVE_HOST}" ""
  ${IfNot} ${Errors}
    StrCpy $AppBrowsers 1
  ${EndIf}
  ClearErrors
  ReadRegStr $0 HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${APP_NATIVE_HOST}" ""
  ${IfNot} ${Errors}
    StrCpy $AppBrowsers 1
  ${EndIf}
  !endif
  app_detect_done:
  Pop $R6
  Pop $R1
  Pop $R0
  Pop $0
FunctionEnd

; The left side: the slide, and the line between the two sides.
Function AppSlidePanel
  Push $0
  !insertmacro AppCtl $AppSlideCtl STATIC ${APP_PIC} 0 0 530 540 ""
  Call AppSlideInit
  ${NSD_OnClick} $AppSlideCtl AppSlideClick
  ; ‹ and › for the keyboard (the mouse clicks them on the slide).
  !insertmacro AppT $AppTx appPrevSlide
  !insertmacro AppProxy $AppPxPrev ${APP_PUSH} 444 492 30 40 $AppTx
  ${NSD_OnClick} $AppPxPrev AppPrevSlide
  !insertmacro AppT $AppTx appNextSlide
  !insertmacro AppProxy $AppPxNext ${APP_PUSH} 474 492 34 40 $AppTx
  ${NSD_OnClick} $AppPxNext AppNextSlide
  !insertmacro AppLabel $AppSepCtl 530 0 1 540 ${APP_LABEL} $AppFBody LN LN ""
  Pop $0
FunctionEnd

Function AppPrevSlide
  Pop $0
  StrCpy $AppAuto 0
  StrCpy $AppDir -1
  IntOp $AppSlide $AppSlide - 1
  ${IfThen} $AppSlide < 1 ${|} StrCpy $AppSlide ${APP_SLIDES} ${|}
  Call AppShowSlide
FunctionEnd

Function AppNextSlide
  Pop $0
  StrCpy $AppAuto 0
  StrCpy $AppDir 1
  IntOp $AppSlide $AppSlide + 1
  ${IfThen} $AppSlide > ${APP_SLIDES} ${|} StrCpy $AppSlide 1 ${|}
  Call AppShowSlide
FunctionEnd

; $AppRet = the current slide's picture.
Function AppSlideBmp
  ${If} $AppSlide = 1
    StrCpy $AppRet $AppS1
  ${ElseIf} $AppSlide = 2
    StrCpy $AppRet $AppS2
  ${ElseIf} $AppSlide = 3
    StrCpy $AppRet $AppS3
  ${ElseIf} $AppSlide = 4
    StrCpy $AppRet $AppS4
  ${Else}
    StrCpy $AppRet $AppS5
  ${EndIf}
FunctionEnd

; $AppRet = a new 24-bit picture the size of a slide (24-bit: the picture control uses it as it
; is, never a copy of its own, so drawing into it shows at once and nothing piles up).
Function AppNewFrame
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  !insertmacro AppScale $0 530
  !insertmacro AppScale $1 540
  IntOp $1 0 - $1
  System::Call "*(i 40, i r0, i r1, &i2 1, &i2 24, i 0, i 0, i 0, i 0, i 0, i 0, i 0) p .r2"
  ; [APP] The picture's handle in $3, its pixels' address in $4 (never the same register).
  System::Call "gdi32::CreateDIBSection(p 0, p r2, i 0, *p .r4, p 0, i 0) p .r3"
  System::Free $2
  StrCpy $AppRet $3
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; Copies the picture $AppA1 (a slide, or $AppFrame) into the drawing $AppA2 (one of the
; memory DCs). The picture is held only for the copy: a picture control can't draw one that is
; held somewhere else.
Function AppCopyIn
  Push $0
  System::Call "gdi32::SelectObject(p $AppDcF, p $AppA1) p .r0"
  System::Call "gdi32::BitBlt(p $AppA2, i 0, i 0, i $AppSW, i $AppSH, p $AppDcF, i 0, i 0, i 0x00CC0020)"
  System::Call "gdi32::SelectObject(p $AppDcF, p r0)"
  Pop $0
FunctionEnd

; $AppOut -> $AppFrame (what the control shows when Windows asks it to repaint).
Function AppFrameSave
  Push $0
  System::Call "gdi32::SelectObject(p $AppDcF, p $AppFrame) p .r0"
  System::Call "gdi32::BitBlt(p $AppDcF, i 0, i 0, i $AppSW, i $AppSH, p $AppDcOut, i 0, i 0, i 0x00CC0020)"
  System::Call "gdi32::SelectObject(p $AppDcF, p r0)"
  Pop $0
FunctionEnd

; Once: the pictures and drawings the slide changes use (kept until Setup closes).
Function AppSlideSetup
  Push $0
  Push $1
  !insertmacro AppScale $AppSW 530
  !insertmacro AppScale $AppSH 540
  ; The picture and the words move; the dots and arrows under them only fade.
  !insertmacro AppScale $AppSTop 496
  !insertmacro AppScale $AppShift 80
  Call AppNewFrame
  StrCpy $AppFrame $AppRet
  Call AppNewFrame
  StrCpy $AppFrom $AppRet
  Call AppNewFrame
  StrCpy $AppTo $AppRet
  Call AppNewFrame
  StrCpy $AppOut $AppRet
  Call AppNewFrame
  StrCpy $AppMix $AppRet
  System::Call "user32::GetDC(p 0) p .r0"
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $AppDcF $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $AppDcFrom $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $AppDcTo $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $AppDcOut $1
  System::Call "gdi32::CreateCompatibleDC(p r0) p .r1"
  StrCpy $AppDcMix $1
  System::Call "user32::ReleaseDC(p 0, p r0)"
  System::Call "gdi32::SelectObject(p $AppDcFrom, p $AppFrom)"
  System::Call "gdi32::SelectObject(p $AppDcTo, p $AppTo)"
  System::Call "gdi32::SelectObject(p $AppDcOut, p $AppOut)"
  System::Call "gdi32::SelectObject(p $AppDcMix, p $AppMix)"
  ; What shows beside a moving slide: the slides' own background (read from the first one, so it
  ; matches the picture exactly).
  StrCpy $AppA1 $AppS1
  StrCpy $AppA2 $AppDcTo
  Call AppCopyIn
  System::Call "gdi32::GetPixel(p $AppDcTo, i 2, i 2) i .r0"
  System::Call "gdi32::CreateSolidBrush(i r0) p .r1"
  System::Call "gdi32::SelectObject(p $AppDcOut, p r1)"
  System::Call "gdi32::SelectObject(p $AppDcMix, p r1)"
  Pop $1
  Pop $0
FunctionEnd

; One step of the change, $AppA1 = how far it is (0 to 1000, eased): what showed ($AppFrom)
; moves off one way and fades out while the new slide ($AppTo) comes in from the other side.
Function AppSlideDraw
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  IntOp $1 $AppA1 * $AppShift
  IntOp $1 $1 / 1000
  IntOp $2 $AppShift - $1
  ${If} $AppDir < 0
    StrCpy $3 $1
    IntOp $4 0 - $2
  ${Else}
    IntOp $3 0 - $1
    StrCpy $4 $2
  ${EndIf}
  ${If} $AppA1 >= 1000
    System::Call "gdi32::BitBlt(p $AppDcOut, i 0, i 0, i $AppSW, i $AppSH, p $AppDcTo, i 0, i 0, i 0x00CC0020)"
  ${Else}
    !insertmacro AppShifted $AppDcOut $AppDcFrom $3
    !insertmacro AppShifted $AppDcMix $AppDcTo $4
    ; BLENDFUNCTION { AC_SRC_OVER, 0, alpha, 0 } as one 32-bit value.
    IntOp $0 $AppA1 * 255
    IntOp $0 $0 / 1000
    ${If} $0 > 0
      IntOp $0 $0 << 16
      System::Call "msimg32::AlphaBlend(p $AppDcOut, i 0, i 0, i $AppSW, i $AppSH, p $AppDcMix, i 0, i 0, i $AppSW, i $AppSH, i r0)"
    ${EndIf}
  ${EndIf}
  ; Straight onto the screen in one copy, and the control isn't asked to paint it again.
  ; DCX_CACHE | DCX_CLIPSIBLINGS: the focus ring over the arrows stays.
  System::Call "user32::GetDCEx(p $AppSlideCtl, p 0, i 0x12) p .r1"
  System::Call "gdi32::BitBlt(p r1, i 0, i 0, i $AppSW, i $AppSH, p $AppDcOut, i 0, i 0, i 0x00CC0020)"
  System::Call "user32::ReleaseDC(p $AppSlideCtl, p r1)"
  System::Call "user32::ValidateRect(p $AppSlideCtl, p 0)"
  Call AppFrameSave
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; A new slide control: it shows the current slide (no move). The slides wait 4.5 s from now.
Function AppSlideInit
  Push $0
  Push $1
  Push $2
  ${If} $AppFrame == ""
  ${OrIf} $AppFrame = 0
    Call AppSlideSetup
  ${EndIf}
  ${NSD_KillTimer} AppFade
  ; With Windows' "Animation effects" off there's nothing to come in: the slide shows at once.
  StrCpy $0 1
  System::Call "user32::SystemParametersInfoW(i 0x1042, i 0, *i .r0, i 0)" ; SPI_GETCLIENTAREAANIMATION
  ${IfThen} $0 = 0 ${|} StrCpy $AppIntro 0 ${|}
  ${If} $AppIntro = 1
    ; The first time: only the slides' background, and the first slide comes in once the
    ; window shows (AppTick).
    System::Call "gdi32::PatBlt(p $AppDcOut, i 0, i 0, i $AppSW, i $AppSH, i 0x00F00021)"
  ${Else}
    Call AppSlideBmp
    StrCpy $AppA1 $AppRet
    StrCpy $AppA2 $AppDcOut
    Call AppCopyIn
  ${EndIf}
  Call AppFrameSave
  SendMessage $AppSlideCtl ${STM_SETIMAGE} 0 $AppFrame
  System::Call "kernel32::GetTickCount() i .r0"
  StrCpy $AppIdleT0 $0
  StrCpy $AppMouseSeen 0
  System::Call "*(i 0, i 0) p .r1"
  System::Call "user32::GetCursorPos(p r1)"
  System::Call "*$1(i .r0, i .r2)"
  System::Free $1
  StrCpy $AppCur0 "$0,$2"
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; The slide changed ($AppSlide, in the direction $AppDir): it slides in (also from the middle
; of a change), or appears at once when Windows' "Animation effects" are off.
Function AppShowSlide
  Push $0
  ; A slide flipped before the first one came in replaces that coming in (it never runs after).
  StrCpy $AppIntro 0
  ; What shows now (maybe half-way through a change) is what moves off.
  StrCpy $AppA1 $AppFrame
  StrCpy $AppA2 $AppDcFrom
  Call AppCopyIn
  Call AppSlideBmp
  StrCpy $AppA1 $AppRet
  StrCpy $AppA2 $AppDcTo
  Call AppCopyIn
  System::Call "kernel32::GetTickCount() i .r0"
  StrCpy $AppIdleT0 $0
  StrCpy $AppFadeT0 $0
  StrCpy $0 1
  System::Call "user32::SystemParametersInfoW(i 0x1042, i 0, *i .r0, i 0)" ; SPI_GETCLIENTAREAANIMATION
  ${If} $0 = 0
    ${NSD_KillTimer} AppFade
    StrCpy $AppA1 1000
    Call AppSlideDraw
  ${Else}
    StrCpy $AppA1 0
    Call AppSlideDraw
    ${NSD_CreateTimer} AppFade 10
  ${EndIf}
  Pop $0
FunctionEnd

; One step of the change (as often as Windows' timer allows): eased out (cubic), done after 650 ms.
; Timed by the clock, so a busy PC skips steps instead of slowing the change down.
Function AppFade
  Push $0
  Push $1
  System::Call "kernel32::GetTickCount() i .r0"
  IntOp $0 $0 - $AppFadeT0
  ${If} $0 >= 650
  ${OrIf} $0 < 0
    ${NSD_KillTimer} AppFade
    StrCpy $AppA1 1000
  ${Else}
    ; 1000 × (1 - (1 - t)³), t = $0 / 650, in thousandths.
    IntOp $1 $0 * 1000
    IntOp $1 $1 / 650
    IntOp $1 1000 - $1
    IntOp $0 $1 * $1
    IntOp $0 $0 / 1000
    IntOp $0 $0 * $1
    IntOp $0 $0 / 1000
    IntOp $AppA1 1000 - $0
  ${EndIf}
  Call AppSlideDraw
  Pop $1
  Pop $0
FunctionEnd

; A click on the slides: ‹ › or a dot flips them, and they stop moving by themselves. A click
; anywhere else on the picture (to bring the window to the front, say) changes nothing.
Function AppSlideClick
  Pop $0
  Push $1
  Push $2
  Push $3
  Push $4
  System::Call "*(i 0, i 0) p .r1"
  System::Call "user32::GetCursorPos(p r1)"
  System::Call "user32::ScreenToClient(p $AppSlideCtl, p r1)"
  System::Call "*$1(i .r2, i .r3)"
  System::Free $1
  ; Back to 100 %.
  IntOp $2 $2 * 96
  IntOp $2 $2 / $AppDpi
  IntOp $3 $3 * 96
  IntOp $3 $3 / $AppDpi
  ${If} $3 >= 492
  ${AndIf} $3 < 532
    ${If} $2 >= 444
    ${AndIf} $2 < 474
      StrCpy $AppAuto 0
      StrCpy $AppDir -1
      IntOp $AppSlide $AppSlide - 1
      ${IfThen} $AppSlide < 1 ${|} StrCpy $AppSlide ${APP_SLIDES} ${|}
      Call AppShowSlide
    ${ElseIf} $2 >= 474
    ${AndIf} $2 < 508
      StrCpy $AppAuto 0
      StrCpy $AppDir 1
      IntOp $AppSlide $AppSlide + 1
      ${IfThen} $AppSlide > ${APP_SLIDES} ${|} StrCpy $AppSlide 1 ${|}
      Call AppShowSlide
    ${ElseIf} $2 < 140
      ; The dots: 7 wide (the current one 20), 6 apart, from x 28.
      StrCpy $1 28
      StrCpy $4 1
      ${Do}
        StrCpy $0 7
        ${IfThen} $4 = $AppSlide ${|} StrCpy $0 20 ${|}
        IntOp $0 $0 + $1
        IntOp $0 $0 + 3
        ${If} $2 < $0
          StrCpy $AppAuto 0
          ; A dot further on comes from the right, an earlier one from the left.
          ${If} $4 <> $AppSlide
            StrCpy $AppDir 1
            ${IfThen} $4 < $AppSlide ${|} StrCpy $AppDir -1 ${|}
            StrCpy $AppSlide $4
            Call AppShowSlide
          ${EndIf}
          ${Break}
        ${EndIf}
        IntOp $1 $0 + 3
        IntOp $4 $4 + 1
      ${LoopUntil} $4 > ${APP_SLIDES}
    ${EndIf}
  ${EndIf}
  Pop $4
  Pop $3
  Pop $2
  Pop $1
FunctionEnd

; Every tenth of a second on the pages with slides: the next slide after 4.5 s without the mouse
; on them, and the buttons' hover pictures. The 4.5 s are read from the clock (a busy PC, Windows
; checking the new Setup say, sends these ticks late, and counting them kept the first slide far
; too long). The mouse holds the slides only once it has moved since they showed: resting where
; Setup was double-clicked, it may just happen to be over them.
Function AppTick
  Push $0
  Push $1
  Push $2
  ; The first slide comes in once Setup's window is on the screen.
  ${If} $AppIntro = 1
    System::Call "user32::IsWindowVisible(p $HWNDPARENT) i .r0"
    ${If} $0 <> 0
      StrCpy $AppIntro 0
      StrCpy $AppDir 1
      Call AppShowSlide
    ${EndIf}
  ${EndIf}
  ${If} $AppMouseSeen <> 1
    System::Call "*(i 0, i 0) p .r1"
    System::Call "user32::GetCursorPos(p r1)"
    System::Call "*$1(i .r0, i .r2)"
    System::Free $1
    ${IfThen} "$0,$2" != $AppCur0 ${|} StrCpy $AppMouseSeen 1 ${|}
  ${EndIf}
  System::Call "kernel32::GetTickCount() i .r0"
  ${If} $AppMouseSeen = 1
    StrCpy $AppA1 $AppSlideCtl
    Call AppMouseOn
    ${IfThen} $AppRet = 1 ${|} StrCpy $AppIdleT0 $0 ${|}
  ${EndIf}
  IntOp $1 $0 - $AppIdleT0
  ${If} $AppAuto = 1
  ${AndIf} $1 >= 4500
    StrCpy $AppDir 1
    IntOp $AppSlide $AppSlide + 1
    ${IfThen} $AppSlide > ${APP_SLIDES} ${|} StrCpy $AppSlide 1 ${|}
    Call AppShowSlide
  ${EndIf}
  !if ${APP_FRENCH} == 1
  ; [APP] The French slides are read one per tick, never while a slide moves (a picture takes a
  ; few hundredths of a second), so the first switch to French is instant.
  ${If} $AppSFReady < ${APP_SLIDES}
  ${AndIf} $AppIntro <> 1
    IntOp $1 $0 - $AppFadeT0
    ${If} $1 > 800
    ${OrIf} $1 < 0
      Call AppReadFrSlide
    ${EndIf}
  ${EndIf}
  !endif
  Call AppButtonsTick
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; Logo, the version under it and the line under that, at the top of the right side.
; $AppA1 = the line under the logo.
Function AppHeader
  Push $0
  Push $1
  StrCpy $1 $AppA1
  !insertmacro AppPicture $0 615 34 160 44 $AppBmpLogo
  !insertmacro AppT $AppTx appVersion
  !insertmacro AppLabel $0 554 80 282 16 0x50000081 $AppFTiny MUT BG $AppTx
  !insertmacro AppLabel $0 554 104 282 18 0x50000081 $AppFSmall MUT BG $1
  Pop $1
  Pop $0
FunctionEnd

; ---------- page 1: everything at once ----------
Function AppMainPage
  StrCpy $PassiveMode $AppPassiveOrig
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $AppDialog
  ${If} $AppDialog == error
    Abort
  ${EndIf}
  Call AppHideNav
  Call AppSetTitle
  !insertmacro AppColors $AppDialog TEXT BG
  StrCpy $AppB1Ctl 0
  StrCpy $AppB2Ctl 0
  Call AppSlidePanel
  StrCpy $AppCanMove 0
  Call AppMainBuild
  Call AppCheckRunning
  ${NSD_CreateTimer} AppTick 100
  ${NSD_SetFocus} $AppPxB1
  nsDialogs::Show
FunctionEnd

; [APP] The right side of the main page, in the language shown (made again in the other one when
; the language changes: AppLangApply). Everything it shows comes from the state ($INSTDIR, the
; switches, $AppMode, $AppRunning...), so nothing the user set is lost.
Function AppMainBuild
  !if ${APP_FRENCH} == 1
  Call AppLangChoice
  !endif
  ; What this Setup does, under the logo.
  Call AppModeTag
  StrCpy $AppA1 $AppRet
  Call AppHeader
  !insertmacro AppLabel $0 554 132 282 1 ${APP_LABEL} $AppFBody LN LN ""
  ; Install in: the folder, and Change. Over an installed ${APP_NAME} it stays where it is (no Change),
  ; unless that folder won't do any more: then Change moves it (AppCheckFolder).
  !insertmacro AppT $AppTx appInstallIn
  !insertmacro AppLabel $0 554 144 282 18 ${APP_LABEL} $AppFBody TEXT BG $AppTx
  !insertmacro AppPicture $0 554 168 282 32 $AppBmpField
  !insertmacro AppLabel $AppPathCtl 584 176 190 16 0x50008080 $AppFSmall TEXT RA $INSTDIR
  !insertmacro AppT $AppTx appChange
  !insertmacro AppLabel $AppChangeCtl 776 176 50 16 0x50000182 $AppFSmall ACCENT RA $AppTx
  ${NSD_OnClick} $AppChangeCtl AppChangeFolder
  !insertmacro AppT $AppTx appChangeName
  !insertmacro AppProxy $AppPxChange ${APP_PUSH} 776 176 50 16 $AppTx
  ${NSD_OnClick} $AppPxChange AppChangeFolder
  !if ${APP_FRENCH} == 1
  ; [APP] The language choice's radio buttons are a group of their own (the arrows move between
  ; English and Français only): the next stand-in starts the next group (WS_GROUP).
  System::Call "user32::GetWindowLongW(p $AppPxChange, i -16) i .r1"
  IntOp $1 $1 | 0x00020000
  System::Call "user32::SetWindowLongW(p $AppPxChange, i -16, i r1)"
  !endif
  !insertmacro AppLabel $0 554 214 282 1 ${APP_LABEL} $AppFBody LN LN ""
  ; The switches (the words can be clicked too).
  !insertmacro AppT $AppTx appDesktop
  !insertmacro AppLabel $0 554 225 240 18 0x50000180 $AppFBody TEXT BG $AppTx
  ${NSD_OnClick} $0 AppToggleDesk
  !insertmacro AppPicture $AppSwDesk 804 225 32 18 $AppBmpOff
  ${NSD_OnClick} $AppSwDesk AppToggleDesk
  !insertmacro AppProxy $AppPxDesk ${APP_CHECK} 804 225 32 18 $AppTx
  ${NSD_OnClick} $AppPxDesk AppToggleDesk
  !insertmacro AppLabel $0 554 254 282 1 ${APP_LABEL} $AppFBody LN LN ""
  !insertmacro AppT $AppTx appStartWithWindows
  !insertmacro AppLabel $0 554 265 240 18 0x50000180 $AppFBody TEXT BG $AppTx
  ${NSD_OnClick} $0 AppToggleStart
  !insertmacro AppPicture $AppSwStart 804 265 32 18 $AppBmpOff
  ${NSD_OnClick} $AppSwStart AppToggleStart
  !insertmacro AppProxy $AppPxStart ${APP_CHECK} 804 265 32 18 $AppTx
  ${NSD_OnClick} $AppPxStart AppToggleStart
  !if ${APP_BROWSER_SWITCH} == 1
  ; [APP] Optional third switch (APP_BROWSER_SWITCH in app.nsh): the browser connection.
  !insertmacro AppLabel $0 554 294 282 1 ${APP_LABEL} $AppFBody LN LN ""
  !insertmacro AppT $AppTx appBrowsers
  !insertmacro AppLabel $0 554 304 240 18 0x50000180 $AppFBody TEXT BG $AppTx
  ${NSD_OnClick} $0 AppToggleBrowsers
  StrCpy $1 $AppTx
  !insertmacro AppT $AppTx appBrowsersSub
  !insertmacro AppLabel $0 554 323 240 16 0x50000180 $AppFTiny MUT BG $AppTx
  ${NSD_OnClick} $0 AppToggleBrowsers
  !insertmacro AppPicture $AppSwBrow 804 313 32 18 $AppBmpOff
  ${NSD_OnClick} $AppSwBrow AppToggleBrowsers
  !insertmacro AppProxy $AppPxBrow ${APP_CHECK} 804 313 32 18 $1
  ${NSD_OnClick} $AppPxBrow AppToggleBrowsers
  !endif
  Call AppShowSwitches
  ; Said here when something stands in the way (the folder) or will happen (${APP_NAME} is open).
  ; [APP] 84 high (up to five lines, also at 150 %: French is longer), ending 4 px over Install.
  !insertmacro AppLabel $AppNoteCtl 554 356 282 84 ${APP_LABEL} $AppFSmall WARN BG ""
  ; Install (Update, Install again, Install this version).
  ${If} $AppMode == "update"
    StrCpy $AppA1 "update"
    !insertmacro AppT $1 appBtnUpdate
  ${ElseIf} $AppMode == "same"
    StrCpy $AppA1 "again"
    !insertmacro AppT $1 appBtnAgain
  ${ElseIf} $AppMode == "older"
    StrCpy $AppA1 "older"
    !insertmacro AppT $1 appBtnOlder
  ${Else}
    StrCpy $AppA1 "install"
    !insertmacro AppT $1 appBtnInstall
  ${EndIf}
  StrCpy $AppA2 282
  Call AppLoadButton1
  !insertmacro AppPicture $AppB1Ctl 554 444 282 40 $AppB1N
  ${NSD_OnClick} $AppB1Ctl AppInstallClick
  !insertmacro AppProxy $AppPxB1 ${APP_PUSH} 554 444 282 40 $1
  ${NSD_OnClick} $AppPxB1 AppInstallClick
  ; By installing, you accept the terms of use (a link). No admin rights needed.
  Call AppTermsLine
  !insertmacro AppRingMake
  Call AppCheckFolder
  ; Over an installed ${APP_NAME} whose folder will do: it stays there (no Change).
  ${If} $AppMode != "install"
  ${AndIf} $AppCanMove <> 1
    ShowWindow $AppChangeCtl ${SW_HIDE}
    ShowWindow $AppPxChange ${SW_HIDE}
    !insertmacro AppScale $1 242
    !insertmacro AppScale $2 16
    System::Call "user32::SetWindowPos(p $AppPathCtl, p 0, i 0, i 0, i r1, i r2, i 0x16)"
    !insertmacro AppColors $AppPathCtl MUT RA
  ${EndIf}
FunctionEnd

!if ${APP_FRENCH} == 1
; [APP] The language choice, top right: "English | Français", right-aligned with the switches. The
; one shown in the text colour, the other in the accent like the window's links, or in grey
; (APP_LANG_OTHER "grey": for an accent close to the text colour). The mouse clicks the word. Under each word its radio button for the keyboard and screen readers: Tab reaches the
; one chosen, the arrows or Space pick, the focus ring shows around the word.
Function AppLangChoice
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  StrCpy $AppA1 $AppFTiny
  StrCpy $AppA2 "${APP_LANG_EN_NAME}"
  Call AppTextWidth
  IntOp $1 $AppRet + 1
  StrCpy $AppA2 "${APP_LANG_FR_NAME}"
  Call AppTextWidth
  IntOp $2 $AppRet + 1
  ; Right to left: Français ends at 836, 8 px, a 1 px line, 8 px, English.
  !insertmacro AppScale $3 836
  IntOp $3 $3 - $2            ; Français' left
  !insertmacro AppScale $4 9
  IntOp $4 $3 - $4            ; the line's left
  !insertmacro AppScale $5 8
  IntOp $5 $4 - $5
  IntOp $5 $5 - $1            ; English's left
  !insertmacro AppScale $6 10 ; top
  !insertmacro AppScale $0 16 ; height
  ; (pixels already: nsDialogs takes them as they are)
  nsDialogs::CreateControl STATIC 0x50000180 0 $5 $6 $1 $0 "${APP_LANG_EN_NAME}"
  Pop $AppLangEnCtl
  SendMessage $AppLangEnCtl ${WM_SETFONT} $AppFTiny 1
  ${NSD_OnClick} $AppLangEnCtl AppLangClickEn
  nsDialogs::CreateControl STATIC 0x50000180 0 $3 $6 $2 $0 "${APP_LANG_FR_NAME}"
  Pop $AppLangFrCtl
  SendMessage $AppLangFrCtl ${WM_SETFONT} $AppFTiny 1
  ${NSD_OnClick} $AppLangFrCtl AppLangClickFr
  ${If} $AppLang == "fr"
    !insertmacro AppColors $AppLangEnCtl ${APP_LANG_OTHER_COLOR} BG
    !insertmacro AppColors $AppLangFrCtl TEXT BG
  ${Else}
    !insertmacro AppColors $AppLangEnCtl TEXT BG
    !insertmacro AppColors $AppLangFrCtl ${APP_LANG_OTHER_COLOR} BG
  ${EndIf}
  ; The stand-ins (BS_AUTORADIOBUTTON; the first starts the group; only the chosen one is a Tab
  ; stop, as in Windows' own radio groups), under the words, the same size.
  ${If} $AppLang == "fr"
    StrCpy $AppA2 0x54020009    ; WS_GROUP
    StrCpy $AppA3 0x54010009    ; WS_TABSTOP
  ${Else}
    StrCpy $AppA2 0x54030009    ; WS_GROUP | WS_TABSTOP
    StrCpy $AppA3 0x54000009
  ${EndIf}
  nsDialogs::CreateControl BUTTON $AppA2 0 $5 $6 $1 $0 "${APP_LANG_EN_NAME}"
  Pop $AppPxEn
  SendMessage $AppPxEn ${WM_SETFONT} $AppFSmall 1
  System::Call "user32::SetWindowPos(p $AppPxEn, p 1, i 0, i 0, i 0, i 0, i 0x13)"
  ${NSD_OnClick} $AppPxEn AppLangClickEn
  nsDialogs::CreateControl BUTTON $AppA3 0 $3 $6 $2 $0 "${APP_LANG_FR_NAME}"
  Pop $AppPxFr
  SendMessage $AppPxFr ${WM_SETFONT} $AppFSmall 1
  System::Call "user32::SetWindowPos(p $AppPxFr, p 1, i 0, i 0, i 0, i 0, i 0x13)"
  ${NSD_OnClick} $AppPxFr AppLangClickFr
  ${If} $AppLang == "fr"
    SendMessage $AppPxFr ${BM_SETCHECK} ${BST_CHECKED} 0
  ${Else}
    SendMessage $AppPxEn ${BM_SETCHECK} ${BST_CHECKED} 0
  ${EndIf}
  ; The line between the words, 10 px high, centred on them.
  !insertmacro AppScale $6 13
  !insertmacro AppScale $0 10
  !insertmacro AppScale $1 1
  nsDialogs::CreateControl STATIC ${APP_LABEL} 0 $4 $6 $1 $0 ""
  Pop $0
  !insertmacro AppColors $0 LN LN
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; A word or a radio button: the language changes a moment later, once Windows is done with the
; click (the control it came from is made again).
Function AppLangClickEn
  Pop $0
  ; A pick, even of the language shown (English over a saved French): it is the user's choice.
  StrCpy $AppLangPicked 1
  StrCpy $AppLangNext "en"
  ${NSD_CreateTimer} AppLangApply 1
FunctionEnd
Function AppLangClickFr
  Pop $0
  StrCpy $AppLangPicked 1
  StrCpy $AppLangNext "fr"
  ${NSD_CreateTimer} AppLangApply 1
FunctionEnd

; The window in the other language, at once: every word of Setup's own, the pictures with words in
; them (the slide shown, the button) and the window's title. The right side is made again by the
; same builder (AppMainBuild), hidden until it is whole (no flash, no jump); the slides, their
; arrows and their timers go on, and the keyboard stays where it was.
Function AppLangApply
  ${NSD_KillTimer} AppLangApply
  ${If} $AppLangNext == $AppLang
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  StrCpy $AppLang $AppLangNext
  ; $LANGUAGE is what Setup saves for the uninstaller once it is done (MUI_LANGDLL_SAVELANGUAGE).
  ${If} $AppLang == "fr"
    StrCpy $LANGUAGE ${LANG_FRENCH}
    StrCpy $AppArtSfx "-fr"
  ${Else}
    StrCpy $LANGUAGE ${LANG_ENGLISH}
    StrCpy $AppArtSfx ""
  ${EndIf}
  Call AppLangTexts
  Call AppUseSlides
  ; Where the keyboard is (that control is made again).
  ; (In a variable: AppMainBuild uses $0 to $2.)
  System::Call "user32::GetFocus() p .r0"
  StrCpy $AppLangFocus ""
  ${If} $0 = $AppPxEn
  ${OrIf} $0 = $AppPxFr
    StrCpy $AppLangFocus "lang"
  ${ElseIf} $0 = $AppPxChange
    StrCpy $AppLangFocus "change"
  ${ElseIf} $0 = $AppPxDesk
    StrCpy $AppLangFocus "desk"
  ${ElseIf} $0 = $AppPxStart
    StrCpy $AppLangFocus "start"
  ${ElseIf} $0 = $AppPxB1
    StrCpy $AppLangFocus "b1"
  ${ElseIf} $0 = $AppPxTerms
    StrCpy $AppLangFocus "terms"
  !if ${APP_BROWSER_SWITCH} == 1
  ${ElseIf} $0 = $AppPxBrow
    StrCpy $AppLangFocus "brow"
  !endif
  ${EndIf}
  ; Kept on a control that stays while the others go (a destroyed control's focus is lost).
  ${If} $AppLangFocus != ""
    System::Call "user32::SetFocus(p $AppPxNext)"
  ${EndIf}
  SendMessage $AppDialog ${WM_SETREDRAW} 0 0
  ; Every control of the right side goes; the slide, its arrows and the line beside it stay.
  System::Call "user32::GetWindow(p $AppDialog, i 5) p .r2" ; GW_CHILD
  ${DoWhile} $2 <> 0
    System::Call "user32::GetWindow(p r2, i 2) p .r3"      ; GW_HWNDNEXT
    ${If} $2 <> $AppSlideCtl
    ${AndIf} $2 <> $AppPxPrev
    ${AndIf} $2 <> $AppPxNext
    ${AndIf} $2 <> $AppSepCtl
      System::Call "user32::DestroyWindow(p r2)"
    ${EndIf}
    StrCpy $2 $3
  ${Loop}
  ; Whether the focus ring shows (Windows hides it again while controls are made).
  SendMessage $AppDialog 0x129 0 0 $3 ; WM_QUERYUISTATE
  ; The big button's pictures (AppMainBuild reads the new language's).
  System::Call "gdi32::DeleteObject(p $AppB1N)"
  System::Call "gdi32::DeleteObject(p $AppB1H)"
  ${If} $AppB1X <> 0
    System::Call "gdi32::DeleteObject(p $AppB1X)"
  ${EndIf}
  Call AppMainBuild
  !insertmacro AppT $AppTx appPrevSlide
  SendMessage $AppPxPrev ${WM_SETTEXT} 0 "STR:$AppTx"
  !insertmacro AppT $AppTx appNextSlide
  SendMessage $AppPxNext ${WM_SETTEXT} 0 "STR:$AppTx"
  Call AppSetTitle
  ; The slide shown, in the new language: one coming in goes on (and ends on it), one standing is
  ; swapped at once. (The first one, still to come in, comes in in the new language.)
  ${If} $AppIntro <> 1
    Call AppSlideBmp
    StrCpy $AppA1 $AppRet
    StrCpy $AppA2 $AppDcTo
    Call AppCopyIn
    System::Call "kernel32::GetTickCount() i .r0"
    IntOp $0 $0 - $AppFadeT0
    ${If} $0 >= 650
    ${OrIf} $0 < 0
      StrCpy $AppA1 1000
      Call AppSlideDraw
    ${EndIf}
  ${EndIf}
  SendMessage $AppDialog ${WM_SETREDRAW} 1 0
  ; RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW | RDW_FRAME
  System::Call "user32::RedrawWindow(p $AppDialog, p 0, p 0, i 0x585)"
  ${If} $AppLangFocus == "lang"
    ${If} $AppLang == "fr"
      SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxFr 1
    ${Else}
      SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxEn 1
    ${EndIf}
  ${ElseIf} $AppLangFocus == "change"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxChange 1
  ${ElseIf} $AppLangFocus == "desk"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxDesk 1
  ${ElseIf} $AppLangFocus == "start"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxStart 1
  ${ElseIf} $AppLangFocus == "b1"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxB1 1
  ${ElseIf} $AppLangFocus == "terms"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxTerms 1
  !if ${APP_BROWSER_SWITCH} == 1
  ${ElseIf} $AppLangFocus == "brow"
    SendMessage $HWNDPARENT ${WM_NEXTDLGCTL} $AppPxBrow 1
  !endif
  ${EndIf}
  IntOp $3 $3 & 1 ; UISF_HIDEFOCUS
  ${If} $3 = 0
    SendMessage $HWNDPARENT 0x127 0x10002 0 ; WM_CHANGEUISTATE, UIS_CLEAR UISF_HIDEFOCUS
  ${EndIf}
  Call AppButtonsTick
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd
!endif

Function AppShowSwitches
  ${If} $AppDesktop = 1
    SendMessage $AppSwDesk ${STM_SETIMAGE} 0 $AppBmpOn
  ${Else}
    SendMessage $AppSwDesk ${STM_SETIMAGE} 0 $AppBmpOff
  ${EndIf}
  ${If} $AppStart = 1
    SendMessage $AppSwStart ${STM_SETIMAGE} 0 $AppBmpOn
  ${Else}
    SendMessage $AppSwStart ${STM_SETIMAGE} 0 $AppBmpOff
  ${EndIf}
  !if ${APP_BROWSER_SWITCH} == 1
  ${If} $AppBrowsers = 1
    SendMessage $AppSwBrow ${STM_SETIMAGE} 0 $AppBmpOn
  ${Else}
    SendMessage $AppSwBrow ${STM_SETIMAGE} 0 $AppBmpOff
  ${EndIf}
  !endif
  ; Screen readers read each switch's on/off from its stand-in.
  SendMessage $AppPxDesk ${BM_SETCHECK} $AppDesktop 0
  SendMessage $AppPxStart ${BM_SETCHECK} $AppStart 0
  !if ${APP_BROWSER_SWITCH} == 1
  SendMessage $AppPxBrow ${BM_SETCHECK} $AppBrowsers 0
  !endif
FunctionEnd

Function AppToggleDesk
  Pop $0
  IntOp $AppDesktop 1 - $AppDesktop
  Call AppShowSwitches
FunctionEnd
Function AppToggleStart
  Pop $0
  IntOp $AppStart 1 - $AppStart
  Call AppShowSwitches
FunctionEnd
!if ${APP_BROWSER_SWITCH} == 1
Function AppToggleBrowsers
  Pop $0
  IntOp $AppBrowsers 1 - $AppBrowsers
  Call AppShowSwitches
FunctionEnd
!endif

; "By installing, you accept the terms of use." (the last words a link), centred, and under it
; "No admin rights needed."
Function AppTermsLine
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  ${If} $AppMode == "update"
    !insertmacro AppT $4 appTermsAUpdate
  ${Else}
    !insertmacro AppT $4 appTermsA
  ${EndIf}
  StrCpy $AppA1 $AppFTiny
  StrCpy $AppA2 $4
  Call AppTextWidth
  StrCpy $1 $AppRet
  !insertmacro AppT $AppTx appTermsLink
  StrCpy $AppA2 $AppTx
  Call AppTextWidth
  StrCpy $2 $AppRet
  !insertmacro AppT $AppTx appTermsB
  StrCpy $AppA2 $AppTx
  Call AppTextWidth
  StrCpy $3 $AppRet
  ; Left edge of the line: the middle of the right side, less half the line.
  IntOp $0 $1 + $2
  IntOp $0 $0 + $3
  !insertmacro AppScale $5 695
  IntOp $6 $0 / 2
  IntOp $5 $5 - $6
  !insertmacro AppScale $6 494
  !insertmacro AppScale $0 16
  ; (pixels already: nsDialogs takes them as they are)
  IntOp $1 $1 + 1
  nsDialogs::CreateControl STATIC ${APP_LABEL} 0 $5 $6 $1 $0 $4
  Pop $4
  SendMessage $4 ${WM_SETFONT} $AppFTiny 1
  !insertmacro AppColors $4 MUT BG
  IntOp $5 $5 + $1
  IntOp $5 $5 - 1
  IntOp $2 $2 + 1
  !insertmacro AppT $AppTx appTermsLink
  nsDialogs::CreateControl STATIC 0x50000180 0 $5 $6 $2 $0 $AppTx
  Pop $4
  SendMessage $4 ${WM_SETFONT} $AppFTiny 1
  !insertmacro AppColors $4 ACCENT BG
  ${NSD_OnClick} $4 AppOpenTerms
  ; Its stand-in for the keyboard (see AppProxy).
  nsDialogs::CreateControl BUTTON ${APP_PUSH} 0 $5 $6 $2 $0 $AppTx
  Pop $AppPxTerms
  System::Call "user32::SetWindowPos(p $AppPxTerms, p 1, i 0, i 0, i 0, i 0, i 0x13)"
  ${NSD_OnClick} $AppPxTerms AppOpenTerms
  IntOp $5 $5 + $2
  IntOp $5 $5 - 1
  IntOp $3 $3 + 1
  !insertmacro AppT $AppTx appTermsB
  nsDialogs::CreateControl STATIC ${APP_LABEL} 0 $5 $6 $3 $0 $AppTx
  Pop $4
  SendMessage $4 ${WM_SETFONT} $AppFTiny 1
  !insertmacro AppColors $4 MUT BG
  ${If} $AppMode == "install"
    !insertmacro AppT $AppTx appNoAdmin
    !insertmacro AppLabel $4 554 512 282 16 0x50000081 $AppFTiny MUT BG $AppTx
  ${EndIf}
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function AppOpenTerms
  Pop $0
  ; The terms in the language shown (APP_TERMS_URL_FR in French, template 1.1.3).
  StrCpy $0 "${APP_TERMS_URL}"
  ${If} $AppLang == "fr"
    StrCpy $0 "${APP_TERMS_URL_FR}"
  ${EndIf}
  ; As the Windows user, never with Setup's rights (Setup may run as administrator).
  nsis_tauri_utils::RunAsUser $0 ""
FunctionEnd

Function AppChangeFolder
  Pop $0
  !insertmacro AppT $AppTx appPickFolder
  nsDialogs::SelectFolderDialog $AppTx $INSTDIR
  Pop $0
  ${If} $0 == error
  ${OrIf} $0 == ""
    Return
  ${EndIf}
  ; Like Windows' own folder pages: ${APP_NAME} gets a folder of its own in the one picked.
  ${GetFileName} $0 $1
  ${If} $1 != "${PRODUCTNAME}"
    StrCpy $1 $0 1 -1
    ${If} $1 == "\"
      StrCpy $0 "$0${PRODUCTNAME}"
    ${Else}
      StrCpy $0 "$0\${PRODUCTNAME}"
    ${EndIf}
  ${EndIf}
  StrCpy $INSTDIR $0
  SendMessage $AppPathCtl ${WM_SETTEXT} 0 "STR:$INSTDIR"
  Call AppCheckFolder
  Call AppCheckRunning
FunctionEnd

; Whether ${APP_NAME} can go in $INSTDIR: sets $AppHeld and says why not under the options.
Function AppCheckFolder
  Push $0
  StrCpy $AppHeld 0
  StrCpy $AppFolderMsg ""
  ${If} $INSTDIR == ""
    !insertmacro AppT $AppFolderMsg appFolderPick
  ${Else}
    ; [APP] Where the installed ${APP_NAME} keeps its notes, and WebView2's data (app.nsh).
    ${If} $INSTDIR == "${APP_DATA_DIR}"
      !insertmacro AppT $AppFolderMsg appFolderData
    ${ElseIf} $INSTDIR == "${APP_WEBVIEW_DIR}"
      !insertmacro AppT $AppFolderMsg appFolderWebview
    ${EndIf}
    ; A portable ${APP_NAME}'s folder: installing there would leave its notes behind.
    ; [APP] Dobo's portable notes: data\ztodo.sqlite next to the exe (paths.rs).
    ${If} $AppFolderMsg == ""
    ${AndIfNot} ${FileExists} "$INSTDIR\${APP_MARKER}"
      ${If} ${FileExists} "$INSTDIR\data\ztodo.sqlite"
        !insertmacro AppT $AppFolderMsg appFolderPortable
      ${EndIf}
    ${EndIf}
    ; A folder this Windows user can't change (or only administrators can), a drive or network
    ; folder that isn't there, a path too long for Windows.
    ${If} $AppFolderMsg == ""
      Call AppCheckDir
      ${If} $AppDirProblem == "admin"
        !insertmacro AppT $AppFolderMsg appFolderAdmin
      ${ElseIf} $AppDirProblem != ""
        ${If} $AppDirWhy == "missing"
          !insertmacro AppT $AppFolderMsg appFolderMissing
        ${ElseIf} $AppDirWhy == "long"
          !insertmacro AppT $AppFolderMsg appFolderLong
        ${Else}
          Push $INSTDIR
          Call AppIsAdminFolder
          Pop $0
          ${If} $0 = 1
            !insertmacro AppT $AppFolderMsg appFolderAdmin
          ${Else}
            !insertmacro AppT $AppFolderMsg appFolderWrite
          ${EndIf}
        ${EndIf}
      ${EndIf}
    ${EndIf}
  ${EndIf}
  ; Over an installed ${APP_NAME}: in another folder than its own, ${APP_NAME} moves there (its uninstaller
  ; removes the old files, PageLeaveReinstall); its own folder that won't do any more can only be
  ; left with Change, said in the note.
  StrCpy $AppMoved 0
  ${If} $AppMode != "install"
    ${If} $INSTDIR != $AppOldDir
      StrCpy $AppMoved 1
    ${ElseIf} $AppFolderMsg != ""
      StrCpy $AppCanMove 1
      !insertmacro AppT $AppTx appMoveHint
      StrCpy $AppFolderMsg "$AppFolderMsg $AppTx"
    ${EndIf}
  ${EndIf}
  ${If} $AppFolderMsg != ""
    StrCpy $AppHeld 1
  ${EndIf}
  Call AppShowNote
  Call AppButtonsTick
  Pop $0
FunctionEnd

; Whether ${APP_NAME} is open there now (it will be closed first: said in the window, not asked).
Function AppCheckRunning
  Push $0
  Push $1
  Push $2
  Push $3
  Push $R0
  StrCpy $AppRunning 0
  ; The installed ${APP_NAME} (where it is, also when it moves), else one in the folder picked.
  StrCpy $3 $INSTDIR
  ${IfThen} $AppMode != "install" ${|} StrCpy $3 $AppOldDir ${|}
  ${If} ${FileExists} "$3\${MAINBINARYNAME}.exe"
    !insertmacro RestartManager_StartSession $R0
    ${If} $R0 != ""
      !insertmacro RestartManager_RegisterFile $R0 "$3\${MAINBINARYNAME}.exe"
      ${If} $0 = 0
        System::Call 'RSTRTMGR::RmGetList(p R0, *i .r1, *i .r2, p 0, *i .r3) i .r0'
        ${If} $0 = ${ERROR_MORE_DATA}
          StrCpy $AppRunning 1
        ${EndIf}
      ${EndIf}
      !insertmacro RestartManager_EndSession $R0
    ${EndIf}
  ${EndIf}
  StrCpy $AppAgreedClose $AppRunning
  Call AppShowNote
  Pop $R0
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function AppShowNote
  ${If} $AppFolderMsg != ""
    !insertmacro AppColors $AppNoteCtl WARN BG
    SendMessage $AppNoteCtl ${WM_SETTEXT} 0 "STR:$AppFolderMsg"
  ${ElseIf} $AppMoved = 1
    !insertmacro AppColors $AppNoteCtl MUT BG
    !insertmacro AppT $AppTx appMoveNote
    ${If} $AppRunning = 1
      !insertmacro AppT $AppRet appIsOpen
      StrCpy $AppTx "$AppTx$\r$\n$AppRet"
    ${EndIf}
    SendMessage $AppNoteCtl ${WM_SETTEXT} 0 "STR:$AppTx"
  ${ElseIf} $AppRunning = 1
    !insertmacro AppColors $AppNoteCtl MUT BG
    !insertmacro AppT $AppTx appIsOpen
    SendMessage $AppNoteCtl ${WM_SETTEXT} 0 "STR:$AppTx"
  ${Else}
    SendMessage $AppNoteCtl ${WM_SETTEXT} 0 "STR:"
  ${EndIf}
  ; The label repaints with its new colour.
  System::Call "user32::InvalidateRect(p $AppNoteCtl, p 0, i 1)"
FunctionEnd

Function AppInstallClick
  Pop $0
  ${If} $AppHeld = 1
    Return
  ${EndIf}
  ; Next: the page's leave function, then the installing.
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd

Function AppMainLeave
  StrCpy $PassiveMode $AppPassiveOrig
  Call AppCheckFolder
  ${If} $AppHeld = 1
    Abort
  ${EndIf}
  !if ${APP_FRENCH} == 1
  ; [APP] With the language untouched (the window starts in English on a first install, in
  ; the saved language over an installed ${APP_NAME}: AppGuiInit), a saved French stays, also after
  ; an uninstall that kept it or over a portable copy: $LANGUAGE is what Setup saves once the
  ; install is done (MUI_LANGDLL_SAVELANGUAGE), and the app gets no --lang (hooks.nsh).
  ${If} $AppLangPicked <> 1
    ReadRegStr $AppTx ${MUI_LANGDLL_REGISTRY_ROOT} "${MUI_LANGDLL_REGISTRY_KEY}" "${MUI_LANGDLL_REGISTRY_VALUENAME}"
    ${If} $AppTx == ${LANG_FRENCH}
      StrCpy $LANGUAGE ${LANG_FRENCH}
    ${EndIf}
  ${EndIf}
  !endif
  ; Over an installed ${APP_NAME}: what Tauri's reinstall page did with its first (recommended)
  ; choice: the same version is installed again over itself; another one is replaced (its
  ; uninstaller removes only its files, see PageLeaveReinstall).
  ${If} $AppMode != "install"
    StrCpy $R0 $AppCmp
    StrCpy $R6 $AppR6
    StrCpy $AppUiChoice 1
    Call PageLeaveReinstall
  ${EndIf}
  ${NSD_KillTimer} AppTick
  ${NSD_KillTimer} AppFade
  ; Installed before the first slide came in: the last page shows it at once.
  StrCpy $AppIntro 0
  !insertmacro AppRingNone
FunctionEnd

; ---------- page 2: installing (Tauri's progress page, in ${APP_NAME}'s look) ----------
Function AppInstShow
  ; A passive run (/P) keeps Windows' plain progress window (nothing of ${APP_NAME}'s look was set up).
  ${If} $AppPassiveOrig = 1
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  Call AppHideNav
  Call AppSetTitle
  FindWindow $0 "#32770" "" $HWNDPARENT
  StrCpy $AppA1 860
  StrCpy $AppA2 540
  Call AppPageFill
  ; The slide stays where it was (still: nothing moves while ${APP_NAME} installs).
  !insertmacro AppRaw $1 $0 STATIC ${APP_PIC} 0 0 530 540 ""
  ${If} $AppSlide = 1
    SendMessage $1 ${STM_SETIMAGE} 0 $AppS1
  ${ElseIf} $AppSlide = 2
    SendMessage $1 ${STM_SETIMAGE} 0 $AppS2
  ${ElseIf} $AppSlide = 3
    SendMessage $1 ${STM_SETIMAGE} 0 $AppS3
  ${ElseIf} $AppSlide = 4
    SendMessage $1 ${STM_SETIMAGE} 0 $AppS4
  ${Else}
    SendMessage $1 ${STM_SETIMAGE} 0 $AppS5
  ${EndIf}
  !insertmacro AppRaw $1 $0 STATIC ${APP_LABEL} 530 0 1 540 ""
  !insertmacro AppColors $1 LN LN
  !insertmacro AppRaw $1 $0 STATIC ${APP_PIC} 615 34 160 44 ""
  SendMessage $1 ${STM_SETIMAGE} 0 $AppBmpLogo
  !insertmacro AppT $AppTx appVersion
  !insertmacro AppRaw $1 $0 STATIC 0x50000081 554 80 282 16 $AppTx
  SendMessage $1 ${WM_SETFONT} $AppFTiny 1
  !insertmacro AppColors $1 MUT BG
  Call AppModeTag
  !insertmacro AppRaw $1 $0 STATIC 0x50000081 554 104 282 18 $AppRet
  SendMessage $1 ${WM_SETFONT} $AppFSmall 1
  !insertmacro AppColors $1 MUT BG
  ${If} $AppMode == "update"
    !insertmacro AppT $2 appUpdating
  ${ElseIf} $AppMode == "same"
    !insertmacro AppT $2 appInstallingAgain
  ${ElseIf} $AppMode == "older"
    !insertmacro AppT $2 appInstallingOlder
  ${Else}
    !insertmacro AppT $2 appInstalling
  ${EndIf}
  !insertmacro AppRaw $1 $0 STATIC 0x50000080 554 210 282 24 $2
  SendMessage $1 ${WM_SETFONT} $AppFTitle 1
  !insertmacro AppColors $1 TEXT BG
  StrCpy $1 554
  StrCpy $2 248
  StrCpy $3 282
  Call AppStyleProgress
  ; Close, for when something goes wrong (AppFail).
  StrCpy $AppA1 "b-done-n-$AppLook$AppArtSfx.png"
  StrCpy $AppA2 282
  StrCpy $AppA3 40
  Call AppImage
  StrCpy $AppFailBmp $AppRet
  StrCpy $1 554
  StrCpy $2 444
  StrCpy $3 282
  Call AppFailClose
  StrCpy $AppInFiles 1
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; ---------- page 3: done ----------
Function AppDonePage
  StrCpy $PassiveMode $AppPassiveOrig
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $AppDialog
  ${If} $AppDialog == error
    Abort
  ${EndIf}
  Call AppHideNav
  Call AppSetTitle
  !insertmacro AppColors $AppDialog TEXT BG
  StrCpy $AppB1Ctl 0
  StrCpy $AppB2Ctl 0
  StrCpy $AppHeld 0
  StrCpy $AppOpen 1
  StrCpy $AppInFiles 0
  Call AppSlidePanel
  !insertmacro AppT $AppA1 appTag
  Call AppHeader
  ${If} $AppMode == "update"
    !insertmacro AppT $1 appDoneUpdate
  ${ElseIf} $AppMode == "same"
    !insertmacro AppT $1 appDoneAgain
  ${ElseIf} $AppMode == "older"
    !insertmacro AppT $1 appDoneOlder
  ${Else}
    !insertmacro AppT $1 appDone
  ${EndIf}
  !insertmacro AppLabel $0 554 210 282 24 ${APP_LABEL} $AppFTitle TEXT BG $1
  ${If} $AppDesktop = 1
    !insertmacro AppT $1 appDoneWhereDesktop
  ${Else}
    !insertmacro AppT $1 appDoneWhere
  ${EndIf}
  !insertmacro AppLabel $0 554 242 282 40 ${APP_LABEL} $AppFBody MUT BG $1
  ; ${APP_EXE} couldn't take the window's choices (start with Windows, the browser connection).
  ${If} $AppChoicesOk == "0"
    ${If} $AppStart = 1
    ${OrIf} $AppBrowsers = 1
      !insertmacro AppT $AppTx appChoicesFailed
      !insertmacro AppLabel $0 554 292 282 72 ${APP_LABEL} $AppFSmall WARN BG $AppTx
    ${EndIf}
  ${EndIf}
  StrCpy $AppA1 "open"
  StrCpy $AppA2 282
  Call AppLoadButton1
  !insertmacro AppPicture $AppB1Ctl 554 444 282 40 $AppB1N
  ${NSD_OnClick} $AppB1Ctl AppOpenClick
  !insertmacro AppT $AppTx appOpenName
  !insertmacro AppProxy $AppPxB1 ${APP_PUSH} 554 444 282 40 $AppTx
  ${NSD_OnClick} $AppPxB1 AppOpenClick
  !insertmacro AppT $AppTx appClose
  !insertmacro AppLabel $0 665 496 60 18 0x50000181 $AppFSmall MUT BG $AppTx
  ${NSD_OnClick} $0 AppCloseClick
  !insertmacro AppT $AppTx appClose
  !insertmacro AppProxy $AppPxB2 ${APP_PUSH} 665 496 60 18 $AppTx
  ${NSD_OnClick} $AppPxB2 AppCloseClick
  !insertmacro AppRingMake
  ; The window's ✕ and Esc close without opening ${APP_NAME}: with Cancel off (as Windows leaves it
  ; after installing), Windows would take ✕ as Next, which opens it.
  GetDlgItem $0 $HWNDPARENT 2
  EnableWindow $0 1
  ${NSD_CreateTimer} AppTick 100
  ${NSD_SetFocus} $AppPxB1
  nsDialogs::Show
FunctionEnd

Function AppOpenClick
  Pop $0
  StrCpy $AppOpen 1
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd
Function AppCloseClick
  Pop $0
  StrCpy $AppOpen 0
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd

; Enter (or Open ${APP_NAME}) opens ${APP_NAME}; Close and the window's ✕ don't.
Function AppDoneLeave
  ${NSD_KillTimer} AppTick
  ${NSD_KillTimer} AppFade
  ${If} $AppOpen = 1
    Call RunMainBinary
  ${EndIf}
FunctionEnd

; ---------- the uninstaller ----------
Function un.AppGuiInit
  StrCpy $AppPassiveOrig $PassiveMode
  ${If} $PassiveMode = 1
    Return
  ${EndIf}
  InitPluginsDir
  SetOutPath "$PLUGINSDIR\art"
  File "${APP_ART}\logo-*.png"
  File "${APP_ART}\on-*.png"
  File "${APP_ART}\off-*.png"
  File "${APP_ART}\b-uninstall-*.png"
  File "${APP_ART}\b-cancel-*.png"
  File "${APP_ART}\b-close-*.png"
  ; [APP] Out of the pictures' folder again: SetOutPath also made it Setup's working folder, and
  ; Windows can't delete a working folder, so NSIS's own clean-up at exit left it behind, empty, in %TEMP%.
  SetOutPath "$TEMP"
  Call un.AppLookInit
  StrCpy $AppA1 460
  StrCpy $AppA2 320
  Call un.AppWindow
  StrCpy $AppA1 "logo-$AppLook.png"
  StrCpy $AppA2 160
  StrCpy $AppA3 44
  Call un.AppImage
  StrCpy $AppBmpLogo $AppRet
  StrCpy $AppA1 "on-$AppLook.png"
  StrCpy $AppA2 32
  StrCpy $AppA3 18
  Call un.AppImage
  StrCpy $AppBmpOn $AppRet
  StrCpy $AppA1 "off-$AppLook.png"
  StrCpy $AppA2 32
  StrCpy $AppA3 18
  Call un.AppImage
  StrCpy $AppBmpOff $AppRet
FunctionEnd

Function un.AppSetTitle
  !insertmacro AppT $AppTx appUnTitle
  SendMessage $HWNDPARENT ${WM_SETTEXT} 0 "STR:$AppTx"
FunctionEnd

Function un.AppTick
  Call un.AppButtonsTick
FunctionEnd

Function un.AppUnPage
  StrCpy $PassiveMode $AppPassiveOrig
  ${If} $PassiveMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $AppDialog
  ${If} $AppDialog == error
    Abort
  ${EndIf}
  Call un.AppHideNav
  Call un.AppSetTitle
  !insertmacro AppColors $AppDialog TEXT BG
  StrCpy $AppHeld 0
  StrCpy $DeleteAppDataCheckboxState 0
  StrCpy $AppDeletePrivate 0
  !insertmacro AppPicture $0 150 18 160 44 $AppBmpLogo
  !insertmacro AppT $AppTx appVersion
  !insertmacro AppLabel $0 24 64 412 16 0x50000081 $AppFTiny MUT BG $AppTx
  !insertmacro AppT $AppTx appUnTitle
  !insertmacro AppLabel $0 24 84 412 24 0x50000081 $AppFTitle TEXT BG $AppTx
  !insertmacro AppT $AppTx appUnText
  !insertmacro AppLabel $0 24 112 412 39 0x50000081 $AppFBody MUT BG $AppTx
  !insertmacro AppLabel $0 24 152 412 1 ${APP_LABEL} $AppFBody LN LN ""
  ; [APP] Two switches: the settings and lists, and Private mode's files apart (they can't be
  ; downloaded again, and the first switch never deletes them).
  !insertmacro AppT $AppTx deleteAppData
  !insertmacro AppLabel $0 24 164 350 18 0x50000180 $AppFBody TEXT BG $AppTx
  ${NSD_OnClick} $0 un.AppToggleData
  !insertmacro AppT $AppTx appUnDataSub
  ; [APP] Without the second switch the room under it is free: the line may wrap to three lines
  ; (from Dobo; French is longer).
  !if ${APP_EXTRA_DATA_SWITCH} == 1
  !insertmacro AppLabel $0 24 183 350 16 0x50000180 $AppFTiny MUT BG $AppTx
  !else
  !insertmacro AppLabel $0 24 183 350 46 0x50000180 $AppFTiny MUT BG $AppTx
  !endif
  ${NSD_OnClick} $0 un.AppToggleData
  !insertmacro AppPicture $AppSwDesk 404 172 32 18 $AppBmpOff
  ${NSD_OnClick} $AppSwDesk un.AppToggleData
  !insertmacro AppT $AppTx deleteAppData
  !insertmacro AppProxy $AppPxDesk ${APP_CHECK} 404 172 32 18 $AppTx
  ${NSD_OnClick} $AppPxDesk un.AppToggleData
  !if ${APP_EXTRA_DATA_SWITCH} == 1
  ; [APP] Optional second switch (APP_EXTRA_DATA_SWITCH in app.nsh): a data folder the first one
  ; never deletes (Pako: Private mode's encrypted files), deleted only when asked for here.
  !insertmacro AppT $AppTx appUnPrivate
  !insertmacro AppLabel $0 24 208 350 18 0x50000180 $AppFBody TEXT BG $AppTx
  ${NSD_OnClick} $0 un.AppTogglePrivate
  !insertmacro AppT $AppTx appUnPrivateSub
  !insertmacro AppLabel $0 24 227 350 16 0x50000180 $AppFTiny MUT BG $AppTx
  ${NSD_OnClick} $0 un.AppTogglePrivate
  !insertmacro AppPicture $AppSwStart 404 216 32 18 $AppBmpOff
  ${NSD_OnClick} $AppSwStart un.AppTogglePrivate
  !insertmacro AppT $AppTx appUnPrivate
  !insertmacro AppProxy $AppPxStart ${APP_CHECK} 404 216 32 18 $AppTx
  ${NSD_OnClick} $AppPxStart un.AppTogglePrivate
  !endif
  StrCpy $AppA1 "cancel"
  StrCpy $AppA2 200
  Call un.AppLoadButton2
  !insertmacro AppPicture $AppB2Ctl 24 262 200 40 $AppB2N
  ${NSD_OnClick} $AppB2Ctl un.AppCancelClick
  !insertmacro AppT $AppTx appCancel
  !insertmacro AppProxy $AppPxB2 ${APP_PUSH} 24 262 200 40 $AppTx
  ${NSD_OnClick} $AppPxB2 un.AppCancelClick
  StrCpy $AppA1 "uninstall"
  StrCpy $AppA2 200
  Call un.AppLoadButton1
  !insertmacro AppPicture $AppB1Ctl 236 262 200 40 $AppB1N
  ${NSD_OnClick} $AppB1Ctl un.AppGoClick
  !insertmacro AppT $AppTx appUninstall
  !insertmacro AppProxy $AppPxB1 ${APP_PUSH} 236 262 200 40 $AppTx
  ${NSD_OnClick} $AppPxB1 un.AppGoClick
  !insertmacro AppRingMake
  ${NSD_CreateTimer} un.AppTick 100
  ${NSD_SetFocus} $AppPxB1
  nsDialogs::Show
FunctionEnd

Function un.AppToggleData
  Pop $0
  IntOp $DeleteAppDataCheckboxState 1 - $DeleteAppDataCheckboxState
  ${If} $DeleteAppDataCheckboxState = 1
    SendMessage $AppSwDesk ${STM_SETIMAGE} 0 $AppBmpOn
  ${Else}
    SendMessage $AppSwDesk ${STM_SETIMAGE} 0 $AppBmpOff
  ${EndIf}
  SendMessage $AppPxDesk ${BM_SETCHECK} $DeleteAppDataCheckboxState 0
FunctionEnd

!if ${APP_EXTRA_DATA_SWITCH} == 1
Function un.AppTogglePrivate
  Pop $0
  IntOp $AppDeletePrivate 1 - $AppDeletePrivate
  ${If} $AppDeletePrivate = 1
    SendMessage $AppSwStart ${STM_SETIMAGE} 0 $AppBmpOn
  ${Else}
    SendMessage $AppSwStart ${STM_SETIMAGE} 0 $AppBmpOff
  ${EndIf}
  SendMessage $AppPxStart ${BM_SETCHECK} $AppDeletePrivate 0
FunctionEnd
!endif

Function un.AppGoClick
  Pop $0
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd
Function un.AppCancelClick
  Pop $0
  SendMessage $HWNDPARENT ${WM_COMMAND} 2 0
FunctionEnd

Function un.AppUnLeave
  ${NSD_KillTimer} un.AppTick
  StrCpy $AppB1Ctl 0
  StrCpy $AppB2Ctl 0
  !insertmacro AppRingNone
FunctionEnd

Function un.AppInstShow
  ; A passive run (/P) keeps Windows' plain progress window (nothing of ${APP_NAME}'s look was set up).
  ${If} $AppPassiveOrig = 1
    Return
  ${EndIf}
  Push $0
  Push $1
  Push $2
  Push $3
  Call un.AppHideNav
  Call un.AppSetTitle
  FindWindow $0 "#32770" "" $HWNDPARENT
  StrCpy $AppA1 460
  StrCpy $AppA2 320
  Call un.AppPageFill
  !insertmacro AppRaw $1 $0 STATIC ${APP_PIC} 150 32 160 44 ""
  SendMessage $1 ${STM_SETIMAGE} 0 $AppBmpLogo
  !insertmacro AppT $AppTx appVersion
  !insertmacro AppRaw $1 $0 STATIC 0x50000081 24 78 412 16 $AppTx
  SendMessage $1 ${WM_SETFONT} $AppFTiny 1
  !insertmacro AppColors $1 MUT BG
  !insertmacro AppT $AppTx appRemoving
  !insertmacro AppRaw $1 $0 STATIC 0x50000081 24 120 412 24 $AppTx
  SendMessage $1 ${WM_SETFONT} $AppFTitle 1
  !insertmacro AppColors $1 TEXT BG
  StrCpy $1 24
  StrCpy $2 160
  StrCpy $3 412
  Call un.AppStyleProgress
  ; Close, for when something goes wrong (AppFail).
  StrCpy $AppA1 "b-close-n-$AppLook$AppArtSfx.png"
  StrCpy $AppA2 412
  StrCpy $AppA3 40
  Call un.AppImage
  StrCpy $AppFailBmp $AppRet
  StrCpy $1 24
  StrCpy $2 256
  StrCpy $3 412
  Call un.AppFailClose
  StrCpy $AppInFiles 1
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function un.AppDonePage
  StrCpy $PassiveMode $AppPassiveOrig
  ${If} $PassiveMode = 1
  ${OrIf} $UpdateMode = 1
    Abort
  ${EndIf}
  nsDialogs::Create 1018
  Pop $AppDialog
  ${If} $AppDialog == error
    Abort
  ${EndIf}
  Call un.AppHideNav
  Call un.AppSetTitle
  !insertmacro AppColors $AppDialog TEXT BG
  StrCpy $AppB2Ctl 0
  !insertmacro AppPicture $0 150 32 160 44 $AppBmpLogo
  !insertmacro AppT $AppTx appVersion
  !insertmacro AppLabel $0 24 78 412 16 0x50000081 $AppFTiny MUT BG $AppTx
  !insertmacro AppT $AppTx appUnDone
  !insertmacro AppLabel $0 24 100 412 24 0x50000081 $AppFTitle TEXT BG $AppTx
  !insertmacro AppT $AppTx appUnDoneText
  !insertmacro AppLabel $0 24 130 412 39 0x50000081 $AppFBody MUT BG $AppTx
  StrCpy $AppA1 "close"
  StrCpy $AppA2 412
  Call un.AppLoadButton1
  !insertmacro AppPicture $AppB1Ctl 24 256 412 40 $AppB1N
  ${NSD_OnClick} $AppB1Ctl un.AppGoClick
  !insertmacro AppT $AppTx appClose
  !insertmacro AppProxy $AppPxB1 ${APP_PUSH} 24 256 412 40 $AppTx
  ${NSD_OnClick} $AppPxB1 un.AppGoClick
  !insertmacro AppRingMake
  StrCpy $AppInFiles 0
  ; The window's ✕ and Esc close it too (with Cancel off, as Windows leaves it after
  ; uninstalling, they did nothing).
  GetDlgItem $0 $HWNDPARENT 2
  EnableWindow $0 1
  ${NSD_CreateTimer} un.AppTick 100
  ${NSD_SetFocus} $AppPxB1
  nsDialogs::Show
FunctionEnd

Function un.AppDoneLeave
  ${NSD_KillTimer} un.AppTick
FunctionEnd
!macroend
