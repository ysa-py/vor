; Vor Desktop — NSIS installer
; ------------------------------------------------------------------
; Builds Vor-desktop-setup.exe (installs to Program Files, Start Menu +
; Desktop shortcuts, Apps & Features uninstall entry; WinDivert runtime
; files are copied alongside the exe and only need elevation when the
; desync engine actually loads the driver).
;
; Build (CI / local, NSIS >= 3.08):
;   makensis -DVERSION=1.0.4 installer\vor.nsi
;
; Inputs:
;   VERSION  (required)  release version, e.g. 1.0.4
;
; All paths use ${__FILEDIR__} (this script's directory) so the installer
; builds correctly no matter which working directory makensis is invoked
; from — the workflow runs it from desktop/.
; ------------------------------------------------------------------

!ifndef VERSION
  !error "VERSION must be defined: makensis -DVERSION=1.0.4 installer/vor.nsi"
!endif

!include "MUI2.nsh"
!include "FileFunc.nsh"

!define PRODUCT_NAME      "Vor Desktop"
!define PRODUCT_PUBLISHER "Vor maintainers"
!define PRODUCT_EXE       "Vor-desktop.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\VorDesktop"
!define DESKTOP_ROOT      "${__FILEDIR__}\.."

Name "${PRODUCT_NAME} ${VERSION}"
OutFile "${DESKTOP_ROOT}\Vor-desktop-setup.exe"
Unicode True
SetCompressor /SOLID lzma
InstallDir "$PROGRAMFILES64\Vor"
RequestExecutionLevel admin

!define MUI_ICON   "${__FILEDIR__}\vor.ico"
!define MUI_UNICON "${__FILEDIR__}\vor.ico"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Vor Desktop (required)"
  SetOutPath "$INSTDIR"
  File "${DESKTOP_ROOT}\${PRODUCT_EXE}"
  File /nonfatal "${DESKTOP_ROOT}\WinDivert.dll"
  File /nonfatal "${DESKTOP_ROOT}\WinDivert64.sys"
  File /nonfatal "${DESKTOP_ROOT}\LICENSE"

  ; Optional engine bundle shipped next to the exe (psiphon3 / tor zips)
  CreateDirectory "$INSTDIR\repo"
  SetOutPath "$INSTDIR\repo"
  File /nonfatal /r "${DESKTOP_ROOT}\repo\*.*"

  CreateDirectory "$SMPROGRAMS\Vor"
  CreateShortcut  "$SMPROGRAMS\Vor\Vor Desktop.lnk" "$INSTDIR\${PRODUCT_EXE}"
  CreateShortcut  "$SMPROGRAMS\Vor\Uninstall Vor Desktop.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut  "$DESKTOP\Vor Desktop.lnk" "$INSTDIR\${PRODUCT_EXE}"

  ; Uninstall registry entry (shows in Apps & Features)
  WriteRegStr SHCTX "${PRODUCT_UNINST_KEY}" "DisplayName"     "${PRODUCT_NAME}"
  WriteRegStr SHCTX "${PRODUCT_UNINST_KEY}" "DisplayVersion"  "${VERSION}"
  WriteRegStr SHCTX "${PRODUCT_UNINST_KEY}" "Publisher"       "${PRODUCT_PUBLISHER}"
  WriteRegStr SHCTX "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr SHCTX "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  ${If} ${FileExists} "$INSTDIR\${PRODUCT_EXE}"
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "${PRODUCT_UNINST_KEY}" "EstimatedSize" $0
  ${EndIf}

  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\Vor\Vor Desktop.lnk"
  Delete "$SMPROGRAMS\Vor\Uninstall Vor Desktop.lnk"
  RMDir  "$SMPROGRAMS\Vor"
  Delete "$DESKTOP\Vor Desktop.lnk"

  Delete "$INSTDIR\${PRODUCT_EXE}"
  Delete "$INSTDIR\WinDivert.dll"
  Delete "$INSTDIR\WinDivert64.sys"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\uninstall.exe"
  RMDir /r "$INSTDIR\repo"
  RMDir  "$INSTDIR"
  DeleteRegKey SHCTX "${PRODUCT_UNINST_KEY}"
SectionEnd
