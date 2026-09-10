; UnifiedShield Inno Setup Installer
; Builds a Windows installer for the UnifiedShield VPN client
;
; IMPORTANT: Cloudflare is BLOCKED in Iran
; Download mirrors use Alibaba Cloud / Tencent CDN

#define AppName "UnifiedShield"
#define AppVersion "1.0.0"
#define AppPublisher "UnifiedShield"
#define AppURL "https://github.com/unifiedshield/unifiedshield-windows"
#define AppExeName "unifiedshield.exe"

[Setup]
AppId={{1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
OutputDir=..\dist
OutputBaseFilename=unifiedshield-{#AppVersion}-setup
SetupIconFile=..\assets\app.ico
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
MinVersion=10.0
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; Language support (including Farsi/Persian for Iranian users)
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "farsi"; MessagesFile: "compiler:Languages\Farsi.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startatboot"; Description: "Start at boot"; GroupDescription: "Startup"; Flags: unchecked

[Files]
; Main executable
Source: "..\build\bin\unifiedshield.exe"; DestDir: "{app}"; Flags: ignoreversion
; Wintun driver
Source: "..\deps\wintun\bin\amd64\wintun.dll"; DestDir: "{app}"; Flags: ignoreversion
; Rust core library
Source: "..\build\lib\unifiedshield_core.dll"; DestDir: "{app}"; Flags: ignoreversion
; Configuration
Source: "..\config\*.json"; DestDir: "{app}\config"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(AppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Registry]
; Start at boot (optional)
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#AppName}"; ValueData: """{app}\{#AppExeName}"""; Flags: uninsdeletevalue; Tasks: startatboot

[UninstallDelete]
Type: filesandirs; Name: "{app}"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
begin
  if CurStep = ssPostInstall then
  begin
    // Install Wintun driver (no admin needed, userspace)
    // The Wintun DLL is simply placed alongside the executable
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    // Stop VPN service if running
    Exec(ExpandConstant('{app}\{#AppExeName}'), '--stop', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
