Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
Name "Charlita"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Charlita"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${SOURCE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\bin\charlita.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "English"
Section "Charlita" SEC_APP
    SetOutPath "$INSTDIR"
    File /r "${STAGE}\*"
    WriteUninstaller "$INSTDIR\Uninstall.exe"
    CreateDirectory "$SMPROGRAMS\Charlita"
    CreateShortcut "$SMPROGRAMS\Charlita\Charlita.lnk" "$INSTDIR\bin\charlita.exe"
    CreateShortcut "$SMPROGRAMS\Charlita\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Charlita" "DisplayName" "Charlita"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Charlita" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Charlita" "UninstallString" '$"$INSTDIR\Uninstall.exe$"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Charlita" "DisplayIcon" "$INSTDIR\bin\charlita.exe"
SectionEnd
Section "Uninstall"
    RMDir /r "$INSTDIR\bin"
    RMDir /r "$INSTDIR\lib"
    RMDir /r "$INSTDIR\qml"
    RMDir /r "$INSTDIR\plugins"
    RMDir /r "$INSTDIR\translations"
    RMDir /r "$INSTDIR\licenses"
    Delete "$INSTDIR\LICENSE"
    Delete "$INSTDIR\THIRD-PARTY-NOTICES.md"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"
    RMDir /r "$SMPROGRAMS\Charlita"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Charlita"
SectionEnd
