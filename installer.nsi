;================================================
; wpush Installer - NSIS 3.x
;================================================

Unicode true
RequestExecutionLevel admin

!define PRODUCT_NAME "wpush"
!define PRODUCT_PUBLISHER "0xA672"

;--- Version from environment or default ---
!ifndef VERSION
  !define VERSION "0.1.4"
!endif
!define PRODUCT_VERSION "${VERSION}"

!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"

;--- General ---
Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "wpush-setup-${PRODUCT_VERSION}-x64.exe"
InstallDir "$PROGRAMFILES64\${PRODUCT_NAME}"
InstallDirRegKey HKLM "Software\${PRODUCT_PUBLISHER}\${PRODUCT_NAME}" "Install_Dir"
ShowInstDetails hide
ShowUnInstDetails hide

;--- MUI Settings ---
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\wpush.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "--help"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

;--- Register PATH helper ---
!macro AddToPathLocal _dir
  Push "${_dir}"
  Call AddToPath
!macroend

!macro RemoveFromPathLocal _dir
  Push "${_dir}"
  Call un.RemoveFromPath
!macroend

;=============================================
; Install Section
;=============================================
Section "Install (required)" SecInstall
  SectionIn RO
  SetOutPath "$INSTDIR"

  File "target\release\wpush.exe"
  File "README.md"
  File "LICENSE"
  File "installer.nsi"

  ;--- Write uninstaller ---
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ;--- Registry (Add/Remove Programs) ---
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                   "DisplayName" "${PRODUCT_NAME} - WSL Git push tool"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                   "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                   "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                   "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                   "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                     "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}" \
                     "NoRepair" 1

  ;--- Add to system PATH ---
  !insertmacro AddToPathLocal "$INSTDIR"

  ;--- Start Menu shortcut ---
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\wpush.lnk" \
                 "$INSTDIR\wpush.exe" "" "$INSTDIR\wpush.exe" 0
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall.lnk" \
                 "$INSTDIR\uninstall.exe" "" "$INSTDIR\uninstall.exe" 0

  ;--- Install dir registry ---
  WriteRegStr HKLM "Software\${PRODUCT_PUBLISHER}\${PRODUCT_NAME}" \
                   "Install_Dir" "$INSTDIR"
SectionEnd

;=============================================
; Uninstall Section
;=============================================
Section "Uninstall"
  ; Remove from PATH
  !insertmacro RemoveFromPathLocal "$INSTDIR"

  ; Remove shortcuts
  RMDir /r "$SMPROGRAMS\${PRODUCT_NAME}"

  ; Remove files
  Delete "$INSTDIR\wpush.exe"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\installer.nsi"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  ; Remove registry keys
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
  DeleteRegKey HKLM "Software\${PRODUCT_PUBLISHER}\${PRODUCT_NAME}"
SectionEnd

;=============================================
; Functions
;=============================================

Function AddToPath
  Exch $0           ; dir to add
  Push $1
  Push $2
  Push $3

  ; Read current system PATH
  ReadRegStr $1 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH"
  StrCpy $2 $0

  ; Check if already present
  Push $1
  Push $2
  Call StrStr
  Pop $3
  IntCmp $3 -1 +2 +2 there
  ; $3 == -1: not found, fall through to add
  ; $3 > -1: found in PATH, skip to there

  ; Append to PATH
  StrCpy $3 $1
  StrCmp $3 "" +2
  StrCpy $3 "$3;$2"
  StrCpy $1 "$3"
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH" "$1"

  ; Notify system
  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment"

there:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function un.RemoveFromPath
  Exch $0           ; dir to remove
  Push $1
  Push $2
  Push $3
  Push $4

  ReadRegStr $1 HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH"
  StrCpy $3 $0

  ; Escape backslashes for StrStr
  Push $1
  Push "$3;"
  Call un.StrStr
  Pop $2
  IntCmp $2 -1 done

  ; Remove entry
  StrLen $4 "$3;"
  StrCpy $1 $1 "" $4
  WriteRegExpandStr HKLM "SYSTEM\CurrentControlSet\Control\Session Manager\Environment" "PATH" "$1"

  SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment"

done:
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

; StrStr - case-sensitive substring search
; Input: top=haystack, second=needle
; Output: top=pos (-1 if not found)
Function StrStr
  Exch $0           ; needle
  Exch
  Exch $1           ; haystack
  Push $2
  Push $3

  StrCpy $2 0
  StrLen $3 $0
  loop:
    StrCpy $2 $1 $3 $2
    StrCmp $2 $0 found
    IntOp $2 $2 + 1
    StrLen $3 $1
    IntCmp $2 $3 done done
  Goto loop

  found:
    StrCpy $0 $2
    Goto end
  done:
    StrCpy $0 -1
  end:
    Pop $3
    Pop $2
    Pop $1
    Exch $0         ; result
FunctionEnd

Function un.StrStr
  Exch $0           ; needle
  Exch
  Exch $1           ; haystack
  Push $2
  Push $3

  StrCpy $2 0
  StrLen $3 $0
  loop:
    StrCpy $2 $1 $3 $2
    StrCmp $2 $0 found
    IntOp $2 $2 + 1
    StrLen $3 $1
    IntCmp $2 $3 done done
  Goto loop

  found:
    StrCpy $0 $2
    Goto end
  done:
    StrCpy $0 -1
  end:
    Pop $3
    Pop $2
    Pop $1
    Exch $0         ; result
FunctionEnd
