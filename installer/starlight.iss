; Inno Setup script for the Starlight installer. Built by the release
; workflow (ISCC is preinstalled on GitHub's Windows runners) alongside the
; portable exe:
;   ISCC.exe /DMyAppVersion=<x.y.z> installer\starlight.iss
;
; Installs per-user (no admin prompt) into {localappdata}\Programs\Starlight,
; which also keeps the in-app self-updater working — it swaps the exe next to
; itself and needs the directory to be user-writable.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0-dev"
#endif

#define MyAppName "Starlight"
#define MyAppExeName "Starlight.exe"
#define MyAppPublisher "All Of Us Mods"
#define MyAppURL "https://github.com/All-Of-Us-Mods/Starlight-PC"

[Setup]
; Never change this AppId — it's how upgrades find the existing install.
AppId={{8E7B1B5C-42D3-4C6A-9A0E-5B1B60E1A7C4}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
DefaultDirName={autopf}\{#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=Starlight-Setup-x86_64
SetupIconFile=..\assets\icons\starlight.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
; Close a running Starlight before replacing the exe.
CloseApplications=yes
LicenseFile=..\LICENSE

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu entry is always created; the desktop shortcut is the optional task.
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; starlight:// deep-link scheme (profile desktop shortcuts use it). The app
; re-registers this on every startup too; creating it here as well means the
; uninstaller cleans it up.
Root: HKCU; Subkey: "Software\Classes\starlight"; ValueType: string; ValueData: "URL:starlight Protocol"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\starlight"; ValueName: "URL Protocol"; ValueType: string; ValueData: ""
Root: HKCU; Subkey: "Software\Classes\starlight\DefaultIcon"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"",0"
Root: HKCU; Subkey: "Software\Classes\starlight\shell\open\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent
